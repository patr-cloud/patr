use std::{net::SocketAddr, pin::pin};

use dashmap::DashMap;
use futures::{future, StreamExt};
use models::{api::workspace::runner::*, rbac::ResourceType};
use tokio::{
	net::TcpListener,
	time::{self, Duration},
};
use tracing::{level_filters::LevelFilter, Dispatch, Level};
use tracing_subscriber::{
	fmt::{format::FmtSpan, Layer as FmtLayer},
	layer::SubscriberExt,
	Layer,
};

use crate::{db, prelude::*, utils::resource_executor::ResourceExecutorTask};

/// All deployment related functions for the runner
mod deployment;

/// The runner is the main struct that is used to run the resources.
///
/// It contains the executor, the database connection pool, and the settings for
/// the runner. The runner is created using the [`Runner::new`] function.
pub struct Runner<E>
where
	E: RunnerExecutor,
{
	/// Runner task registry
	registry: DashMap<Uuid, ResourceExecutorTask>,
	/// State and configuration for the runner
	state: AppState<E>,
}

impl<E> Runner<E>
where
	E: RunnerExecutor + Clone + 'static,
{
	/// Initializes the runner. This function will create a new
	/// database connection pool and set up the global default subscriber for
	/// the runner. It returns an instance of the runner.
	pub async fn init() -> Result<Self, RunnerError> {
		let config = RunnerSettings::<E::Settings>::parse(&E::runner_internal_name())?;

		tracing::dispatcher::set_global_default(Dispatch::new(
			tracing_subscriber::registry().with(
				FmtLayer::new()
					.with_span_events(FmtSpan::NONE)
					.event_format(
						tracing_subscriber::fmt::format()
							.with_ansi(true)
							.with_file(false)
							.without_time()
							.compact(),
					)
					.with_filter(
						tracing_subscriber::filter::Targets::new()
							.with_target(E::runner_internal_name(), LevelFilter::TRACE)
							.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE),
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

		let database = db::connect(&config.database).await?;

		let state = AppState { database, config };

		db::initialize(&state).await?;

		Ok(Self {
			registry: DashMap::new(),
			state,
		})
	}

	/// Run the runner. This function will start the runner and run the server
	/// and the resource reconciliation. It will return a result with the error
	/// if the runner fails to start. The runner will run until the exit signal
	/// is received.
	pub async fn run(self) -> Result<(), RunnerError> {
		let tcp_listener = TcpListener::bind(self.state.config.bind_address).await?;

		E::initialize(&self.state.config).await?;

		future::join3(
			self.run_server(tcp_listener),
			self.sync_local_database(),
			self.monitor_resources(),
		)
		.await;

		info!("Runner stopped. Waiting for server to exit...");
		info!("Server exited. Exiting runner");
		Ok(())
	}

	/// Run the server. This function will start the server and listen for
	/// incoming HTTP connections. It will return a result with the error if the
	/// server fails to start. The server will run until the exit signal is
	/// received.
	async fn run_server(&self, tcp_listener: TcpListener) {
		info!(
			"Listening for connections on http://{}",
			tcp_listener.local_addr().unwrap()
		);

		axum::serve(
			tcp_listener,
			crate::routes::setup_routes(&self.state)
				.await
				.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.with_graceful_shutdown(exit_signal())
		.await
		.expect("Unable to start server");
	}

	/// Sync the local database with the upstream APIs. This function will
	/// connect to the server and listen for messages from the server. It will
	/// notify the runner to reconcile the resources that are changed on the
	/// server. This function will only run if the runner is running in managed
	/// mode.
	async fn sync_local_database(&self) {
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
			return;
		};

		info!("Syncing local database with upstream APIs");

		let exit_signal = &mut pin!(exit_signal());
		debug!("Exit signal listener started");

		info!("Connecting to the server");
		// Connect to the server infinitely until the exit signal is received
		'main: loop {
			let Some(response) = client::stream_request(
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
					.body(Default::default())
					.build(),
			)
			.some_if_not_exit(exit_signal)
			.await
			else {
				// None signifies exit signal
				warn!("Exit signal received. Stopping runner");
				break 'main;
			};
			info!("Connected to the server");

			let Ok(stream) = response
				.inspect_err(|err| {
					error!("Failed to connect to the server: {:?}", err);
					error!("Retrying in 5 second");
				})
				.map_err(|err| err.body)
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				if time::sleep(Duration::from_secs(5))
					.some_if_not_exit(exit_signal)
					.await
					.is_none()
				{
					// None signifies exit signal
					break 'main;
				};
				continue 'main;
			};

			info!("Reconciling all resources before starting");
			// Reconcile all resources at the start (or when reconnecting to the websocket)
			while let Err(err) = self.resync_all().await {
				error!("Failed to resync all resources: {:?}", err);
				error!("Retrying in 1 second");
				time::sleep(Duration::from_secs(1)).await;
			}

			let mut pinned_stream = pin!(stream);

			'message: loop {
				let Some(reconcile_message) =
					pinned_stream.next().some_if_not_exit(exit_signal).await
				else {
					// None signifies exit signal
					break 'main;
				};

				match reconcile_message {
					Some(Ok(response)) => {
						self.handle_server_message(response).await;
					}
					Some(Err(err)) => {
						// Data from the websocket failed
						error!("Failed to connect to the server: {:?}", err);
						error!("Retrying in 1 second");
						// Retry after 1 second, but break if the exit signal is received
						if time::sleep(Duration::from_secs(1))
							.some_if_not_exit(exit_signal)
							.await
							.is_none()
						{
							// None signifies exit signal
							break 'main;
						};

						break 'message;
					}
					None => {
						// Websocket disconnected. Reconnect
						error!("Connection to server closed");
						error!("Retrying in 2 seconds");
						// Retry after 2 seconds, but break if the exit signal is received
						if time::sleep(Duration::from_secs(2))
							.some_if_not_exit(exit_signal)
							.await
							.is_none()
						{
							// None signifies exit signal
							break 'main;
						};

						break 'message;
					}
				}
			}
		}
	}

	async fn monitor_resources(&self) {
		info!("Monitoring all running resources");
		loop {
			// Every few seconds, ensure that all resources in self.registry are
			// running and is in sync with the resources in the database
		}
	}

	/// Resync all the resources that the runner is responsible for. This
	/// function will sync the resources that are running with the resources
	/// that should be running.
	async fn resync_all(&self) -> Result<(), RunnerError> {
		// Reconcile all resources
		self.resync_all_deployments().await?;

		Ok(())
	}

	/// Handle a message from the server. This function will handle the message
	/// from the server and run the reconciliation for the resource that the
	/// message is for.
	async fn handle_server_message(&self, msg: StreamRunnerDataForWorkspaceServerMsg) {
		info!("Handling server message: {:?}", msg);
		// if this resource is already queued for reconciliation, remove that
		let resource_id = get_resource_id_from_message(&msg);

		match msg.resource_type() {
			ResourceType::Deployment => {
				self.reconcile_deployment(resource_id).await;
			}
			_ => {
				warn!("Unknown resource type: {:?}", msg);
			}
		}
	}
}

/// Listen for the exit signal and stop the runner when the signal is received.
#[tracing::instrument]
async fn exit_signal() {
	let ctrl_c = async {
		tokio::signal::ctrl_c()
			.await
			.expect("Failed to listen for SIGINT")
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
		_ = ctrl_c => (),
		_ = terminate => (),
	}
	info!("Shutdown signal received, shutting down server gracefully");
}

/// For a given message, get the resource ID from the message
fn get_resource_id_from_message(message: &StreamRunnerDataForWorkspaceServerMsg) -> Uuid {
	use StreamRunnerDataForWorkspaceServerMsg::*;
	match message {
		DeploymentCreated { deployment, .. } => deployment.id,
		DeploymentUpdated { deployment, .. } => deployment.id,
		DeploymentDeleted { id } => *id,
	}
}
