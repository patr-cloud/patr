#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! Common utilities for the runner. This library contains all the things you
//! will need to make a runner. All it needs are the implementations of how the
//! runner handles the resources (such as deployments, databases, etc).

/// The client for the Patr API to get runner data for a given workspace.
mod client;
/// The configuration for the runner.
mod config;
/// The executor module contains the trait that the runner needs to implement
/// to run the resources.
mod executor;

use std::{marker::PhantomData, str::FromStr, time::Duration};

use futures::StreamExt;
use models::{api::workspace::runner::*, utils::WebSocketUpgrade};
use prelude::*;
use tracing::{level_filters::LevelFilter, Dispatch, Level};
use tracing_subscriber::{
	fmt::{format::FmtSpan, Layer as FmtLayer},
	layer::SubscriberExt,
	Layer,
};

/// The prelude module contains all the things you need to import to get
/// started with the runner.
pub mod prelude {
	pub use models::prelude::*;

	pub use crate::{config::*, executor::RunnerExecutor};

	/// The type of the database connection. A mutable reference to this should
	/// be used as the parameter for database functions, since it accepts both a
	/// connection and a transaction.
	///
	/// Example:
	/// ```rust
	/// pub fn database_fn(connection: &mut DatabaseConnection) {
	///     // Do something with `connection` ....
	/// }
	/// ```
	pub type DatabaseConnection = <DatabaseType as sqlx::Database>::Connection;

	/// The type of the database connection pool. This is used to create
	/// transactions and connections to the database.
	pub type DatabaseConnectionPool = sqlx::Pool<DatabaseType>;

	/// The type of the database transaction. This is used in requests to
	/// rollback or commit transactions based on how an endpoint responds. This
	/// currently has a static lifetime, implying that only transactions from a
	/// pooled connection is allowed.
	pub type DatabaseTransaction = sqlx::Transaction<'static, DatabaseType>;

	/// The type of the database. This is currently set to [`sqlx::Postgres`].
	/// A type alias is used here so that it can be referenced everywhere easily
	pub type DatabaseType = sqlx::Sqlite;
}

/// The runner is the main struct that is used to run the resources. It contains
/// the executor, the database connection pool, and the settings for the runner.
/// The runner is created using the [`Runner::new`] function.
pub struct Runner<'de, E>
where
	E: RunnerExecutor,
{
	/// The executor for the runner. This is the main trait that the runner
	/// needs to implement to run the resources.
	executor: E,
	/// The database connection pool for the runner. This is used to connect to
	/// the database that holds information about the resources that the runner
	/// is running.
	database: DatabaseConnectionPool,
	/// The settings for the runner. This contains the workspace ID, the runner
	/// ID, the API token, the environment, and any additional settings that the
	/// runner needs.
	settings: RunnerSettings<E::Settings<'de>>,
}

impl<E> Runner<'_, E>
where
	E: RunnerExecutor,
{
	/// Create a new runner with the given executor. This function will create a
	/// new database connection pool and set up the global default subscriber
	/// for the runner.
	pub async fn new(executor: E) -> Self {
		let settings = get_runner_settings();

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
							.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE),
					)
					.with_filter(LevelFilter::from_level(
						if settings.environment == RunningEnvironment::Development {
							Level::TRACE
						} else {
							Level::DEBUG
						},
					)),
			),
		))
		.expect("Failed to set global default subscriber");

		let database = DatabaseConnectionPool::connect("sqlite::memory:")
			.await
			.expect("unable to connect to the database");

		Self {
			executor,
			database,
			settings,
		}
	}

	/// Run the runner. This function will start the runner and run the
	/// resources that the runner is responsible for. This function will run
	/// forever until the runner is stopped.
	pub async fn run(self) {
		let authorization =
			BearerToken::from_str(&self.settings.api_token).expect("Failed to parse Bearer token");
		let user_agent = UserAgent::from_str(&format!("TODO")).expect("Failed to parse User-Agent");

		loop {
			client::stream_request(
				ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
					.path(StreamRunnerDataForWorkspacePath {
						workspace_id: self.settings.workspace_id,
						runner_id: self.settings.runner_id,
					})
					.headers(StreamRunnerDataForWorkspaceRequestHeaders {
						authorization,
						user_agent,
					})
					.query(())
					.body(WebSocketUpgrade(PhantomData))
					.build(),
			)
			.await
			.map_err(|err| err.body)
			.expect("Cannot stream request")
			.for_each(|response| async move {
				use StreamRunnerDataForWorkspaceServerMsg::*;
				match response {
					DeploymentCreated {
						deployment,
						running_details,
					} => todo!(),
					DeploymentUpdated {
						deployment,
						running_details,
					} => todo!(),
					DeploymentDeleted { id } => todo!(),
				}
			})
			.await;
		}
	}
}

/// Listen for the exit signal and stop the runner when the signal is received.
#[tracing::instrument]
pub async fn exit_signal() {
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
	info!("signal received, shutting down server gracefully");
}
