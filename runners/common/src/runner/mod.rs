use std::{fs::Permissions, net::SocketAddr, os::unix::fs::PermissionsExt, sync::OnceLock};

use dashmap::DashMap;
use futures::{FutureExt, future};
use tempfile::TempDir;
use tokio::{
	fs,
	net::TcpListener,
	sync::{mpsc, watch},
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

use crate::{db, prelude::*, resource_executor::ResourceExecutorTask};

/// The global cancellation token that will be used to cancel the tasks
/// when the runner is stopped. This token will be used to cancel all the
/// tasks that are running in the runner.
#[doc(hidden)]
pub(super) static GLOBAL_CANCEL_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

/// The part of the runner that handles the Cloudflare tunnel.
/// This is used to expose the runner to the internet when the runner is
/// running in managed mode and the runner exposure type requires a tunnel.
mod cloudflare_tunnel;
/// The part of the runner that syncs the local database with the upstream APIs.
///
/// This is only used when the runner is running in managed mode. This connects
/// to the server and listens for changes to the resources. When a change is
/// detected, it updates the local database and notifies the resource monitor
/// to reconcile the resources.
mod database_sync;
/// All deployment related functions for the runner
mod deployment;
/// The part of the runner that monitors resources.
///
/// The job of this is simple: Make sure that for every resource in the
/// database, there is a task running. If the database updates, notify the task
/// that something has changed. That's it. It's the task's job to update the
/// resource. As long as it's running, we are happy. So NO updating the
/// resource here whatsoever. All that happens in the executor task.
mod monitor_resources;
/// The part of the runner that handles the embedded nginx server.
/// This is used to proxy requests from the tunnel to the actual deployments.
mod nginx;

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
	E: RunnerExecutor + Send + Sync + 'static,
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

		// Create the channel for deployment status updates
		let (task_status_sender, _) = mpsc::unbounded_channel();
		let (nginx_reload_sender, _) = watch::channel(());

		let state = AppState {
			database,
			config,
			runner_state,
			task_status_sender,
			nginx_reload_sender,
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
		self.state.task_status_sender = sender;

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
			self.run_server(tcp_listener).inspect(|_| {
				debug!("Server has shut down");
			}),
			self.sync_local_database(receiver).inspect(|_| {
				debug!("Database sync has stopped");
			}),
			self.run_cloudflare_tunnel().inspect(|_| {
				debug!("Cloudflare tunnel has stopped");
			}),
			self.run_nginx().inspect(|_| {
				debug!("Nginx has stopped");
			}),
			self.monitor_resources().inspect(|_| {
				debug!("Resource monitor has stopped");
			}),
		)
		.await;

		server_setup?;

		info!("Runner stopped. Waiting for server to exit...");

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
	GLOBAL_CANCEL_TOKEN
		.get_or_init(CancellationToken::new)
		.cancel();
	info!("Shutdown signal received, shutting down server gracefully");
}
