use std::{
	fs::Permissions,
	io,
	net::SocketAddr,
	os::unix::fs::PermissionsExt,
	pin::pin,
	process::Stdio,
	sync::OnceLock,
};

use dashmap::DashMap;
use fslock::LockFile;
use futures::{
	StreamExt,
	future::{self, Either},
};
use models::{api::workspace::runner::*, utils::WebSocketUpgrade};
use tempfile::TempDir;
use tokio::{
	fs,
	net::TcpListener,
	process::Command,
	sync::mpsc::{self, UnboundedReceiver},
	task,
	time::{self, Duration},
};
use tokio_util::sync::CancellationToken;
use tracing::{Dispatch, Level, level_filters::LevelFilter};
use tracing_subscriber::{
	Layer,
	fmt::{Layer as FmtLayer, format::FmtSpan},
	layer::SubscriberExt,
};

use crate::{
	db,
	prelude::*,
	utils::{client::make_request, resource_executor::ResourceExecutorTask},
};

/// The global cancellation token that will be used to cancel the tasks
/// when the runner is stopped. This token will be used to cancel all the
/// tasks that are running in the runner.
pub(super) static GLOBAL_CANCEL_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

/// All deployment related functions for the runner
mod deployment;

/// The runner is the main struct that is used to run the resources.
///
/// It contains the executor, the database connection pool, and the settings for
/// the runner. The runner is created using the [`Runner::new`] function.
pub struct Runner<E>
where
	E: RunnerExecutor + Send + 'static,
{
	/// Runner task registry
	registry: DashMap<Uuid, ResourceExecutorTask<E>>,
	/// State and configuration for the runner
	state: AppState<E>,
	/// Temporary directory for the runner to store binary artifacts
	temp_dir: TempDir,
}

impl<E> Runner<E>
where
	E: RunnerExecutor + Send + 'static,
{
	/// If this is set to true, the runner will use the embedded binaries for
	/// cloudflared and nginx instead of using the system binaries. This is
	/// useful for keeping the code for the embedded binaries before the feature
	/// gets released.
	const USE_EMBEDDED_BINARIES: bool = false;

	/// Initializes the runner. This function will create a new
	/// database connection pool and set up the global default subscriber for
	/// the runner. It returns an instance of the runner.
	#[instrument]
	pub async fn init() -> Result<Self, RunnerError> {
		let config = RunnerSettings::<E::Settings>::parse(&E::runner_internal_name())?;

		Self::init_with_config(config).await
	}

	/// Initializes the runner with the given configuration. This function will
	/// set up the global default subscriber for the runner, connect to the
	/// database, and initialize the runner state. It returns an instance of the
	/// runner.
	#[instrument(skip(config))]
	pub async fn init_with_config(
		config: RunnerSettings<E::Settings>,
	) -> Result<Self, RunnerError> {
		tracing::dispatcher::set_global_default(Dispatch::new(
			tracing_subscriber::registry().with(
				FmtLayer::new()
					.with_span_events(FmtSpan::NONE)
					.event_format(
						tracing_subscriber::fmt::format()
							.with_ansi(true)
							.with_file(false)
							.without_time()
							.with_target(false)
							.with_source_location(false)
							.compact(),
					)
					.with_filter(
						tracing_subscriber::filter::Targets::new()
							.with_target(E::runner_internal_name(), LevelFilter::TRACE)
							.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE)
							.with_target("frontend", LevelFilter::TRACE),
					)
					.with_filter(LevelFilter::from_level(
						if config.environment == RunningEnvironment::Development {
							Level::TRACE
						} else {
							Level::DEBUG
						},
					)),
			),
		))?;

		trace!("Initialized global logger");

		let database = db::connect(&config.database).await?;

		let runner_state = E::initialize(&config).await?;

		let (change_publisher, _) = mpsc::unbounded_channel();

		let state = AppState {
			database,
			config,
			runner_state,
			change_publisher,
		};

		db::initialize(&state).await?;

		Ok(Self {
			registry: DashMap::new(),
			state,
			temp_dir: TempDir::with_prefix("patr").map_err(RunnerError::ServerSetupError)?,
		})
	}

	/// Run the runner. This function will start the runner and run the server
	/// and the resource reconciliation. It will return a result with the error
	/// if the runner fails to start. The runner will run until the exit signal
	/// is received.
	#[instrument(skip(self))]
	pub async fn run(mut self) -> Result<!, RunnerError> {
		debug!("Attempting to listen on {}", self.state.config.bind_address);
		let tcp_listener = TcpListener::bind(self.state.config.bind_address)
			.await
			.map_err(RunnerError::ServerSetupError)?;

		let (sender, receiver) = mpsc::unbounded_channel();
		self.state.change_publisher = sender;

		task::spawn(async move {
			info!("Listening for exit signal");
			exit_signal().await;

			warn!("Exit signal received. Gracefully stopping runner...");
			GLOBAL_CANCEL_TOKEN
				.get_or_init(CancellationToken::new)
				.cancel();

			time::sleep(Duration::from_secs(5)).await;
			info!("Runner has not quit gracefully for 5 seconds");
			info!("Send the exit signal again to force quit (data integrity not guaranteed)");

			exit_signal().await;
			std::process::exit(1);
		});

		if Self::USE_EMBEDDED_BINARIES {
			// Write the cloudflare tunnel binary to the temp directory
			fs::write(
				self.temp_dir.path().join("cloudflared"),
				Binaries::get("cloudflared")
					.expect("Failed to get cloudflared binary")
					.data,
			)
			.await
			.map_err(RunnerError::CloudflareTunnelSetupError)?;
			fs::set_permissions(
				self.temp_dir.path().join("cloudflared"),
				Permissions::from_mode(0o755),
			)
			.await
			.map_err(RunnerError::CloudflareTunnelSetupError)?;

			// Write the nginx binary to the temp directory
			fs::write(
				self.temp_dir.path().join("nginx"),
				Binaries::get("nginx")
					.expect("Failed to get cloudflared binary")
					.data,
			)
			.await
			.map_err(RunnerError::NginxSetupError)?;
			fs::set_permissions(
				self.temp_dir.path().join("nginx"),
				Permissions::from_mode(0o755),
			)
			.await
			.map_err(RunnerError::NginxSetupError)?;
		}

		let (server_setup, sync_database, run_tunnel, run_nginx, resource_monitor) = future::join5(
			self.run_server(tcp_listener),
			self.sync_local_database(),
			self.run_cloudflare_tunnel(),
			self.run_nginx(),
			self.monitor_resources(receiver),
		)
		.await;

		server_setup?;

		info!("Runner stopped. Waiting for server to exit...");

		GLOBAL_CANCEL_TOKEN
			.get_or_init(CancellationToken::new)
			.cancel();
		for (_, task) in self.registry {
			_ = task.stop().await;
		}

		info!("Server exited. Exiting runner");
		sync_database
			.or(run_tunnel)
			.or(run_nginx)
			.or(resource_monitor)
	}

	/// Run the server. This function will start the server and listen for
	/// incoming HTTP connections. It will return a result with the error if the
	/// server fails to start. The server will run until the exit signal is
	/// received.
	#[instrument(skip(self))]
	async fn run_server(&self, tcp_listener: TcpListener) -> Result<(), RunnerError> {
		info!(
			"Listening for connections on http://{}",
			tcp_listener
				.local_addr()
				.map_err(RunnerError::ServerSetupError)?
		);

		axum::serve(
			tcp_listener,
			crate::routes::setup_routes(&self.state)
				.await
				.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.with_graceful_shutdown(exit_signal())
		.await
		.map_err(RunnerError::ServerSetupError)
	}

	/// Sync the local database with the upstream APIs. This function will
	/// connect to the server and listen for messages from the server. It will
	/// notify the runner to reconcile the resources that are changed on the
	/// server. This function will only run if the runner is running in managed
	/// mode. This function will exit if the exit signal is received.
	#[instrument(skip(self))]
	async fn sync_local_database(&self) -> Result<!, RunnerError> {
		let RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token,
			user_agent,
		} = self.state.config.mode.clone()
		else {
			// If the runner is running in self-hosted mode, return early. The run function
			// uses a join of all the futures so early return here will not stop the runner
			// from running
			debug!("Runner is running in self-hosted mode. Skipping sync");
			return Err(RunnerError::Unsupported);
		};

		info!("Syncing local database with upstream APIs");

		info!("Connecting to the server");
		// Connect to the server infinitely until the exit signal is received
		'main: loop {
			let response = client::stream_request(
				ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
					.path(StreamRunnerDataForWorkspacePath {
						workspace_id,
						runner_id,
					})
					.headers(StreamRunnerDataForWorkspaceRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.query(())
					.body(WebSocketUpgrade::default())
					.build(),
			)
			.with_cancel_check()
			.await?;

			let Ok(stream) = response
				.inspect_err(|err| {
					error!("Failed to connect to the server: {:?}", err);
					error!("Retrying in 5 second");
				})
				.map_err(|err| err.body)
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue 'main;
			};
			info!("Connected to the server");

			trace!("Syncing all resources before starting streaming");
			while let Err(err) = self
				.resync_all_resources_with_upstream(
					workspace_id,
					runner_id,
					&api_token,
					&user_agent,
				)
				.await
			{
				error!("Failed to sync all resources: {:?}", err);
				error!("Retrying in 1 second");
				// Retry after 1 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(1))
					.with_cancel_check()
					.await?;
			}
			info!("All resources synced successfully");

			let mut pinned_stream = pin!(stream);

			'message: loop {
				let reconcile_message = pinned_stream.next().with_cancel_check().await?;

				match reconcile_message {
					Some(Ok(response)) => {
						let mut try_count = 0;
						while let Err(err) = self.handle_server_message(response.clone()).await {
							// Failed to handle the message. Retry after 1 second
							error!("Failed to handle the message: {err}");
							warn!("Retrying in 1 second...");
							time::sleep(Duration::from_secs(1))
								.with_cancel_check()
								.await?;
							try_count += 1;

							if try_count >= 5 {
								error!("Handing server message failed more than 5 times.");
								error!("Restarting connection to server");
								continue 'main;
							}
						}
					}
					Some(Err(err)) => {
						// Data from the websocket failed
						error!("Failed to connect to the server: {:?}", err);
						error!("Retrying in 1 second");

						// Retry after 1 second, but break if the exit signal is received
						time::sleep(Duration::from_secs(1))
							.with_cancel_check()
							.await?;

						break 'message;
					}
					None => {
						// Websocket disconnected. Reconnect
						error!("Connection to server closed");
						error!("Retrying in 2 seconds");
						// Retry after 2 seconds, but break if the exit signal is received
						time::sleep(Duration::from_secs(2))
							.with_cancel_check()
							.await?;

						break 'message;
					}
				}
			}
		}
	}

	/// Run the cloudflare tunnel. This function will start the cloudflare
	/// tunnel and listen for incoming connections. It will return a result
	/// with the error if the tunnel fails to start. The tunnel will run until
	/// the exit signal is received.
	#[instrument(skip(self))]
	async fn run_cloudflare_tunnel(&self) -> Result<!, RunnerError> {
		let RunnerMode::Managed {
			workspace_id,
			runner_id,
			api_token,
			user_agent,
		} = self.state.config.mode.clone()
		else {
			// If the runner is running in self-hosted mode, return early. The run function
			// uses a join of all the futures so early return here will not stop the runner
			// from running
			debug!("Runner is running in self-hosted mode. Skipping cloudflare tunnel");
			return Err(RunnerError::Unsupported);
		};

		let runner_exposure_type = E::runner_exposure_type();

		if !runner_exposure_type.requires_tunnel() {
			return Err(RunnerError::Unsupported);
		}

		info!("Running cloudflare tunnel to expose the runner");
		loop {
			let tunnel_token = make_request(
				ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
					.path(GetIngressTokenForRunnerPath {
						workspace_id,
						runner_id,
					})
					.query(())
					.headers(GetIngressTokenForRunnerRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.body(GetIngressTokenForRunnerRequest)
					.build(),
			)
			.with_cancel_check()
			.await?;

			let Ok(tunnel_token) = tunnel_token
				.inspect_err(|err| {
					error!("Failed to connect to the server: {:?}", err);
					error!("Retrying in 5 second");
				})
				.map_err(|err| err.body)
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue;
			};

			let Ok(mut child) = Command::new("cloudflared")
				.arg("tunnel")
				.arg("--logfile")
				.arg("./data/cloudflared.log")
				.arg("run")
				.arg("--token")
				.arg(tunnel_token.body.token)
				.env(
					"PATH",
					format!(
						"{}:{}",
						self.temp_dir.path().display(),
						std::env::var("PATH").unwrap_or_default()
					),
				)
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.kill_on_drop(true)
				.spawn()
				.inspect_err(|err| {
					error!("Failed to start cloudflare tunnel: {:?}", err);
					error!("Retrying in 5 second");
				})
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue;
			};

			let status = match child.wait().with_cancel_check().await {
				Ok(status) => status,
				Err(err) => {
					// Exit signal received. Kill the child process and exit
					child
						.kill()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					child
						.wait()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					return Err(err);
				}
			};

			let Ok(status) = status.inspect_err(|err| {
				error!("Error waiting for cloudflared process: {}", err);
				error!("Retrying in 5 second");
			}) else {
				// Retry after 5 seconds, but break if the exit signal is received
				if let Err(RunnerError::ExitSignalReceived) = time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await
				{
					// Exit signal received. Kill the child process and exit
					child
						.kill()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					child
						.wait()
						.await
						.map_err(RunnerError::CloudflareTunnelExecError)?;
					return Err(RunnerError::ExitSignalReceived);
				}
				continue;
			};

			if status.success() {
				warn!("Cloudflare tunnel exited successfully");
				future::ready(()).with_cancel_check().await?;
				warn!("This should not happen. Restarting tunnel");
			} else {
				error!("Cloudflare tunnel exited with status: {}", status);
				error!("Retrying in 1 second");
				// Retry after a second, but break if the exit signal is received
				time::sleep(Duration::from_secs(1))
					.with_cancel_check()
					.await?;
			}
		}
	}

	/// Run nginx. This function will start nginx and listen for incoming
	/// connections. It will return a result with the error if nginx fails
	/// to start. Nginx will run until the exit signal is received.
	#[instrument(skip(self))]
	async fn run_nginx(&self) -> Result<!, RunnerError> {
		fs::create_dir_all("./data/nginx")
			.await
			.map_err(RunnerError::NginxSetupError)?;

		fs::write(
			constants::NGINX_CONFIG_PATH,
			include_str!(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/../../assets/runner/nginx.conf"
			)),
		)
		.await
		.map_err(RunnerError::NginxSetupError)?;

		loop {
			let mut lock_file = LockFile::open(constants::NGINX_LOCK_FILE_PATH)
				.map_err(RunnerError::NginxSetupError)?;

			if !lock_file.owns_lock() {
				// remove the nginx socket file if it exists based on a lockfile
				let locked = lock_file.try_lock().map_err(RunnerError::NginxSetupError)?;
				if !locked {
					error!("Failed to acquire lock for nginx. Another instance might be running");
					time::sleep(Duration::from_secs(5))
						.with_cancel_check()
						.await?;
					continue;
				}

				if fs::try_exists(constants::NGINX_SOCKET_PATH)
					.await
					.map_err(RunnerError::NginxSetupError)?
				{
					warn!("Removing existing nginx socket file");
					fs::remove_file(constants::NGINX_SOCKET_PATH)
						.await
						.map_err(RunnerError::NginxSetupError)
						.inspect_err(|err| {
							error!("Failed to remove nginx socket file: {:?}", err);
						})?;
				}
			}

			let Ok(mut child) = Command::new("nginx")
				.arg("-g")
				.arg("daemon off;")
				.arg("-p")
				.arg(".")
				.arg("-e")
				.arg("./data/nginx/error.log")
				.arg("-c")
				.arg(constants::NGINX_CONFIG_PATH)
				.env(
					"PATH",
					format!(
						"{}:{}",
						self.temp_dir.path().display(),
						std::env::var("PATH").unwrap_or_default()
					),
				)
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.kill_on_drop(true)
				.spawn()
				.inspect_err(|err| {
					error!("Failed to start nginx: {:?}", err);
				})
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				time::sleep(Duration::from_secs(5))
					.with_cancel_check()
					.await?;
				continue;
			};

			let status = child.wait().await.map_err(RunnerError::NginxExecError)?;

			if status.success() {
				warn!("Nginx exited successfully");
				future::ready(()).with_cancel_check().await?;
				warn!("This should not happen. Restarting nginx");
			} else {
				error!("Nginx exited with status: {}", status);
			}
		}
	}

	/// Reload nginx configuration. This function will send a reload signal to
	/// nginx, causing it to reload its configuration. It will return a result
	/// with the error if nginx fails to reload. This function is useful when
	/// the nginx configuration is changed and needs to be reloaded without
	/// restarting the nginx process.
	#[instrument(skip(self))]
	async fn reload_nginx(&self) -> Result<(), RunnerError> {
		let output = Command::new("nginx")
			.arg("-s")
			.arg("reload")
			.arg("-p")
			.arg(".")
			.arg("-e")
			.arg("./data/nginx/error.log")
			.arg("-c")
			.arg(constants::NGINX_CONFIG_PATH)
			.env(
				"PATH",
				format!(
					"{}:{}",
					self.temp_dir.path().display(),
					std::env::var("PATH").unwrap_or_default()
				),
			)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.await
			.inspect_err(|err| {
				error!("Failed to reload nginx: {:?}", err);
			})
			.map_err(RunnerError::NginxExecError)?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			error!("Nginx reload failed: {}", stderr);
			return Err(RunnerError::NginxExecError(io::Error::new(
				io::ErrorKind::Other,
				"Failed to reload nginx",
			)));
		}

		Ok(())
	}

	/// Monitor the resources and make sure that they are running. This function
	/// will listen for changes in the resources and make sure that they are
	/// running. The job of this function is to make sure that whatever is
	/// running is exactly as per what's in the database.
	#[instrument(skip(self))]
	async fn monitor_resources(
		&self,
		mut change_publisher: UnboundedReceiver<StreamRunnerDataForWorkspaceServerMsg>,
	) -> Result<!, RunnerError> {
		let full_sync_interval = if cfg!(debug_assertions) {
			Duration::from_secs(10)
		} else {
			Duration::from_secs(60 * 10) // 10 minutes
		};

		let mut sleep_future = Box::pin(time::sleep(full_sync_interval));

		// Remember: The point of this loop is not to update the database or the
		// resource. Our job is simple: Make sure that for every resource in the
		// database, there is a task running. It's the task's job to update the
		// resource. As long as it's running, we are happy. So NO updating the
		// resource here whatsoever. All that happens in the task.
		loop {
			match future::select(sleep_future, pin!(change_publisher.recv())).await {
				Either::Left(((), _)) => {
					// Regularly (every 10 minutes in prod and 10 seconds in dev) reconcile all the
					// deployments. Check all resources in the local database and make sure they are
					// running on the runner.
					let Ok(()) = self.reconcile_resources().await else {
						time::sleep(Duration::from_secs(1))
							.with_cancel_check()
							.await?;
						sleep_future = Box::pin(time::sleep(Duration::from_millis(0)));
						continue;
					};
					sleep_future = Box::pin(time::sleep(full_sync_interval));
				}
				Either::Right((update, next_sleep)) => {
					use StreamRunnerDataForWorkspaceServerMsg::*;

					sleep_future = next_sleep;

					let Some(update) = update else {
						continue;
					};

					match update {
						DeploymentCreated {
							deployment,
							running_details: _,
						} |
						DeploymentUpdated {
							deployment,
							running_details: _,
						} => {
							self.upsert_running_deployment(deployment.id).await;
						}
						DeploymentDeleted { id } => {
							if let Err(err) = self.delete_running_deployment(id).await {
								error!("Failed to delete deployment: {err}");
								_ = self.state.change_publisher.send(DeploymentDeleted { id });
							}
						}
					}
				}
			}

			time::sleep(full_sync_interval).with_cancel_check().await?;
		}
	}

	/// Resync all the resources that the runner is responsible for. This
	/// function will sync the local database with the upstream API, making sure
	/// both are in sync.
	#[instrument(skip(self, api_token))]
	async fn resync_all_resources_with_upstream(
		&self,
		workspace_id: Uuid,
		runner_id: Uuid,
		api_token: &BearerToken,
		user_agent: &UserAgent,
	) -> Result<(), RunnerError> {
		// Reconcile all resources
		self.resync_all_deployments_with_upstream(workspace_id, runner_id, api_token, user_agent)
			.await?;

		Ok(())
	}

	/// Handle a message from the server. This function will handle the message
	/// from the server and run the reconciliation for the resource that the
	/// message is for.
	#[instrument(skip(self))]
	async fn handle_server_message(
		&self,
		msg: StreamRunnerDataForWorkspaceServerMsg,
	) -> Result<(), RunnerError> {
		use StreamRunnerDataForWorkspaceServerMsg::*;

		let mut transaction = self.state.database.begin().await?;

		match msg {
			DeploymentCreated {
				deployment,
				running_details,
			} => {
				self.create_deployment_in_database(&mut transaction, deployment, running_details)
					.await?;
			}
			DeploymentUpdated {
				deployment,
				running_details,
			} => {
				self.delete_deployment_in_database(&mut transaction, deployment.id)
					.await?;
				self.create_deployment_in_database(&mut transaction, deployment, running_details)
					.await?;
			}
			DeploymentDeleted { id } => {
				self.delete_deployment_in_database(&mut transaction, id)
					.await?;
			}
		}

		Ok(())
	}
}

/// Listen for the exit signal and stop the runner when the signal is received.
#[instrument]
async fn exit_signal() {
	let ctrl_c = async {
		tokio::signal::ctrl_c()
			.await
			.expect("Failed to listen for SIGINT");
	};

	#[cfg(unix)]
	let terminate = async {
		tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
			.expect("failed to install signal handler")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		() = ctrl_c => (),
		() = terminate => (),
	}
	info!("Shutdown signal received, shutting down server gracefully");
}
