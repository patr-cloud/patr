use std::{net::SocketAddr, sync::OnceLock};

use opentelemetry_sdk::logs::SdkLoggerProvider;
use ractor::Actor;
use tokio::{
	net::TcpListener,
	task,
	time::{self, Duration},
};
use tokio_util::sync::CancellationToken;

use crate::{
	actors::runner_supervisor::{RunnerSupervisor, RunnerSupervisorArgs},
	db,
	prelude::*,
};

/// The global cancellation token that will be used to cancel the tasks
/// when the runner is stopped. This token will be used to cancel all the
/// tasks that are running in the runner.
#[doc(hidden)]
pub(super) static GLOBAL_CANCEL_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

/// The runner is the main struct that is used to run the resources.
///
/// It contains the executor, the database connection pool, and the settings for
/// the runner. The runner is created using the [`Runner::new`] function.
pub struct Runner<E>
where
	E: RunnerExecutor + Send + 'static,
{
	/// Database connection pool for SQLite access.
	database: sqlx::Pool<DatabaseType>,
	/// Runner configuration (settings, mode, bind address, etc.).
	config: RunnerSettings<E::Settings>,
	/// Executor-specific initialized state (e.g. Docker client).
	runner_state: E::InitializedState,
	/// OTLP logger provider for managed-mode log export (None in self-hosted)
	logger_provider: Option<SdkLoggerProvider>,
}

impl<E> Runner<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
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
		// sqlx (ring) and reqwest (aws-lc-rs) both enable their crypto backend on
		// rustls. When both are compiled in, rustls can't auto-detect — pick one.
		let _ = rustls::crypto::ring::default_provider().install_default();

		// Set up OTLP log layer for managed mode
		let logger_provider = crate::utils::observability::setup_tracing::<E>(&config)?;

		trace!("Initialized global logger");

		let database = db::connect(&config.database).await?;

		// Run schema migrations BEFORE E::initialize so the executor sees an
		// up-to-date schema if it ever needs to read from the database.
		db::initialize(&database).await?;

		let runner_state = E::initialize(&config).await?;

		Ok(Self {
			database,
			config,
			runner_state,
			logger_provider,
		})
	}

	/// Run the runner. This function will start the actor tree, HTTP server,
	/// and block until the exit signal is received.
	#[instrument(skip(self))]
	pub async fn run(mut self) -> Result<!, RunnerError> {
		debug!("Attempting to listen on {}", self.config.bind_address);
		let tcp_listener = TcpListener::bind(self.config.bind_address)
			.await
			.map_err(RunnerError::ServerSetupError)?;

		// Spawn the exit signal handler
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

		// Start the actor tree. The RunnerSupervisor spawns and supervises
		// the ResourceSupervisor and WebSocketActor as children. HTTP routes
		// send messages to the RunnerSupervisor, which forwards them.
		let (root_ref, root_handle) = RunnerSupervisor::<E>::spawn(
			Some("runner-supervisor".to_string()),
			RunnerSupervisor::new(),
			RunnerSupervisorArgs {
				config: self.config.clone(),
				database: self.database.clone(),
				runner_state: self.runner_state.clone(),
			},
		)
		.await
		.map_err(RunnerError::host)?;

		// Build AppState for the HTTP server
		let state = AppState::<E> {
			database: self.database.clone(),
			config: self.config.clone(),
			runner_state: self.runner_state.clone(),
			supervisor_ref: root_ref.clone(),
		};

		// Run the HTTP server
		info!(
			"Listening for connections on http://{}",
			tcp_listener
				.local_addr()
				.map_err(RunnerError::ServerSetupError)?
		);

		axum::serve(
			tcp_listener,
			crate::routes::setup_routes(&state)
				.await
				.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.with_graceful_shutdown(
			GLOBAL_CANCEL_TOKEN
				.get_or_init(CancellationToken::new)
				.cancelled(),
		)
		.await
		.map_err(RunnerError::ServerSetupError)?;

		info!("HTTP server stopped");

		// Stop the actor tree — RunnerSupervisor stops its children.
		root_ref.stop(Some("runner shutting down".to_string()));
		root_handle.await.map_err(RunnerError::host)?;

		// Flush OTLP logs before exiting
		if let Some(provider) = self.logger_provider.take() {
			crate::utils::observability::flush_observability(provider);
		}

		info!("Runner exited");
		// The server has stopped, so we need to exit. This is a divergent
		// function (returns !) so we need to loop or exit.
		std::process::exit(0);
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
