use std::{mem::DropGuard, sync::OnceLock};

use futures::{FutureExt, future};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use tokio::{
	sync::mpsc,
	task,
	time::{self, Duration},
};
use tokio_util::sync::CancellationToken;

use crate::{db, prelude::*};

/// The controller reconciles owners: for each one, it applies the objects the
/// local state expects and deletes the ones it no longer does.
///
/// Its queue holds owners whose objects may be out of date, never commands.
/// The WebSocket connection enqueues the owners an upstream change affects,
/// the backend's events enqueue the owners they touch, and a full reconcile of
/// every owner runs on a timer, so nothing is lost if an event is missed. After
/// each deployment's reconcile, it records the deployment's observed status and
/// has the WebSocket connection publish it.
pub mod controller;

/// Shared database utility functions for writing the local SQLite state and
/// loading it back. Used by the WebSocket connection to apply server messages,
/// and by the controller to load the state it reconciles.
pub mod db_helpers;

/// The WebSocket connection to the upstream Patr API.
///
/// It receives server-pushed resource changes, writes them to the local SQLite
/// database, and enqueues the owners they affect with the controller. In the
/// reverse direction, it publishes observed statuses. On connection failure,
/// it reconnects with exponential backoff. Periodic full resyncs re-fetch
/// everything from the API to catch any missed changes.
pub mod websocket;

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
	/// Runner configuration (settings, credentials, database, etc.).
	config: RunnerSettings<E::Settings>,
	/// The executor that runs resources on the backend.
	executor: E,
	/// Whether the database was created fresh, in which case nothing is
	/// reconciled until the first resync fills it.
	fresh_database: bool,
	/// OTLP logger provider for managed-mode log export
	logger_provider: SdkLoggerProvider,
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
		let fresh_database = db::initialize(&database, &config.clone().into_base()).await?;

		let executor = E::initialize(&config).await?;

		Ok(Self {
			database,
			config,
			executor,
			fresh_database,
			logger_provider,
		})
	}

	/// Run the runner. This function will start the controller and the
	/// WebSocket connection, and block until the exit signal is received.
	#[instrument(skip(self))]
	pub async fn run(self) -> Result<!, RunnerError> {
		let Self {
			database,
			config,
			executor,
			fresh_database,
			logger_provider,
		} = self;

		// Flush OTLP logs however this returns: an exit signal while connecting,
		// an error, or a panic
		let flush_logs = DropGuard::new(
			logger_provider,
			crate::utils::observability::flush_observability,
		);

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

		// Connect to the server and resync before starting anything, so the
		// controller starts from up-to-date data. A fresh database expects
		// nothing, and the controller would reap every object on the host, so
		// it keeps trying until the resync succeeds. An existing database
		// tries once, and starts from what it has if that fails.
		let mut connection = websocket::connect::<E>(&config).with_cancel_check().await?;
		let mut backoff = Duration::from_secs(1);
		loop {
			if let Ok(stream) = &mut connection {
				if let Err(err) = websocket::handle_full_resync(&config, &database)
					.with_cancel_check()
					.await?
				{
					error!("Failed to sync all resources: {:?}", err);
				} else {
					info!("All resources synced successfully");
					if let Err(err) = websocket::send_observed_statuses(&database, stream).await {
						error!("Failed to send client message: {:?}", err);
					}
					break;
				}
			}
			if !fresh_database {
				// If this isn't a fresh db, we already have some data to work with. Break so that
				// we can work off of that in the controller instead of holding up the runner boot
				// for the connection
				break;
			}

			// Retry after the backoff, but break if the exit signal is received
			error!("Retrying in {backoff:?}");
			time::sleep(backoff).with_cancel_check().await?;
			backoff = (backoff * 2).min(Duration::from_secs(60));
			if connection.is_err() {
				connection = websocket::connect::<E>(&config).with_cancel_check().await?;
			}
		}

		let (controller_tx, controller_rx) = mpsc::unbounded_channel();
		let (websocket_tx, websocket_rx) = mpsc::unbounded_channel();

		// Each of these handles its own failures, and only returns once the exit
		// signal is received.
		let (controller, websocket) = future::join(
			controller::run(
				controller_rx,
				websocket_tx,
				executor.clone(),
				config.clone(),
				database.clone(),
			)
			.inspect(|_| {
				debug!("Controller has stopped");
			}),
			websocket::run(
				connection,
				websocket_rx,
				controller_tx,
				executor,
				config,
				database,
			)
			.inspect(|_| {
				debug!("Database sync has stopped");
			}),
		)
		.await;

		info!("Runner exited");
		let result = controller.or(websocket);
		// `exit` skips destructors, so the logs are flushed first
		drop(flush_logs);
		// A graceful exit isn't a failure, so it quits with code 0
		if let Err(RunnerError::ExitSignalReceived) = result {
			std::process::exit(0);
		}
		result
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
	info!("Shutdown signal received, shutting down runner gracefully");
}
