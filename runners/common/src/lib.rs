#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]
#![feature(never_type)]

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

use std::{collections::HashMap, str::FromStr, time::Duration};

use futures::TryStreamExt;
use models::{
	api::workspace::{deployment::*, runner::*},
	utils::WebSocketUpgrade,
};
use prelude::*;
use tokio::time;
use tracing::{level_filters::LevelFilter, Dispatch, Level};
use tracing_subscriber::{
	fmt::{format::FmtSpan, Layer as FmtLayer},
	layer::SubscriberExt,
	Layer,
};

/// The prelude module contains all the things you need to import to get
/// started with the runner.
pub mod prelude {
	pub use macros::{query, version};
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
	/// The state of the runner. This is used to store the list of deployments,
	/// databases, etc. that the runner is running.
	deployments: HashMap<Uuid, (Deployment, DeploymentRunningDetails)>,
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

		let deployments = HashMap::new();

		Self {
			executor,
			database,
			settings,
			deployments,
		}
	}

	/// Run the runner. This function will start the runner and run the
	/// resources that the runner is responsible for. This function will run
	/// forever until the runner is stopped.
	pub async fn run(mut self) -> Result<!, ErrorType> {
		let authorization = BearerToken::from_str(&self.settings.api_token)?;
		let user_agent = UserAgent::from_str(concat!(
			env!("CARGO_PKG_NAME"),
			"/",
			env!("CARGO_PKG_VERSION"),
		))?;
		let workspace_id = self.settings.workspace_id;
		let runner_id = self.settings.runner_id;

		loop {
			let Ok(stream) = client::stream_request(
				ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
					.path(StreamRunnerDataForWorkspacePath {
						workspace_id,
						runner_id,
					})
					.headers(StreamRunnerDataForWorkspaceRequestHeaders {
						authorization: authorization.clone(),
						user_agent: user_agent.clone(),
					})
					.query(())
					.body(WebSocketUpgrade::new())
					.build(),
			)
			.await
			.inspect_err(|err| {
				error!("Failed to connect to the server: {:?}", err);
				error!("Retrying in 5 second");
			})
			.map_err(|err| err.body) else {
				time::sleep(Duration::from_secs(5)).await;
				continue;
			};

			let Ok(()) = stream
				.try_for_each(|response| async {
					use StreamRunnerDataForWorkspaceServerMsg::*;
					match response {
						DeploymentCreated {
							deployment,
							running_details,
						} => self.create_deployment(deployment, running_details).await,
						DeploymentUpdated {
							deployment,
							running_details,
						} => self.update_deployment(deployment, running_details).await,
						DeploymentDeleted { id } => self.delete_deployment(id).await,
					}
				})
				.await
				.inspect_err(|err| {
					error!("Failed to connect to the server: {:?}", err);
					error!("Retrying in 1 second");
				})
			else {
				time::sleep(Duration::from_secs(1)).await;
				continue;
			};
		}
	}

	/// Create a new deployment in this runner
	async fn create_deployment(
		&mut self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> Result<(), ErrorType> {
		self.deployments.insert(
			deployment.id,
			(deployment.data.clone(), running_details.clone()),
		);
		self.executor
			.create_deployment(deployment, running_details)
			.await?;
		Ok(())
	}

	/// Update a deployment in this runner
	async fn update_deployment(
		&mut self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> Result<(), ErrorType> {
		self.deployments
			.entry(deployment.id)
			.or_insert((deployment.data.clone(), running_details.clone()));
		self.executor
			.update_deployment(deployment, running_details)
			.await?;
		Ok(())
	}

	/// Delete a deployment in this runner
	async fn delete_deployment(&mut self, id: Uuid) -> Result<(), ErrorType> {
		self.deployments.remove(&id);
		self.executor.delete_deployment(id).await?;
		Ok(())
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
