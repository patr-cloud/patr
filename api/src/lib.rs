#![feature(impl_trait_in_assoc_type)]

//! The main API server for Patr.

/// This module contains the main application logic. Most of the app requests,
/// states, and mounting of endpoints are done here
pub mod app;
/// This module contains the database connection logic, as well as all the
/// ORM entities.
pub mod db;
/// Database migrations organized by version. Uses `inventory` for automatic
/// registration and tracks individual migrations in a `_migrations` table.
pub mod migrations;
/// This module contains the models used by the API. These are the structs that
/// are used for encoding and decoding things that are not a part of the API
/// (eg, JWT).
pub mod models;
/// This module contains the Redis connection and all utilities to set and get
/// data in Redis.
pub mod redis;
/// This module is used to listen for changes in the database and publish them
/// to Redis. This is used for the real-time updates on stream requests.
pub mod redis_publisher;
/// This module contains the routes for the API. This is where the endpoints
/// are mounted.
pub mod routes;
/// This module contains all the utilities used by the API. This includes things
/// like the config parser, the [`tower::Layer`]s that are used to parse the
/// requests.
pub mod utils;
/// The background worker that processes long-running tasks, or cron jobs for
/// frequently used tasks. This uses [`apalis`] under the hood and all the
/// worker tasks are defined here.
pub mod worker;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use macros::query;
	pub use models::{
		AppResponse,
		ErrorType,
		ProcessedApiRequest,
		api::{ApiEndpoint, WithId},
		rbac::{
			BillingPermission,
			ContainerRegistryRepositoryPermission,
			DatabasePermission,
			DeploymentPermission,
			DnsRecordPermission,
			DomainPermission,
			ManagedURLPermission,
			Permission,
			RunnerPermission,
			SecretPermission,
			StaticSitePermission,
		},
		utils::{
			BearerToken,
			ClientType,
			DockerContentDigest,
			DockerDistributionApiVersion,
			ListResourceQuery,
			LoginId,
			OneOrMore,
			OptionalHeader,
			Uuid,
		},
	};
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{AppRequest, AppState, AuthenticatedAppRequest, UnprocessedAppRequest},
		models::ip_lookup as ip,
		redis,
		utils::{self, EitherExt, RouterExt, TimeoutExt, WorkerExt, constants},
		worker::mailer::*,
	};

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

	/// The type of the database transaction. This is used in requests to
	/// rollback or commit transactions based on how an endpoint responds. This
	/// currently has a static lifetime, implying that only transactions from a
	/// pooled connection is allowed.
	pub type DatabaseTransaction = sqlx::Transaction<'static, DatabaseType>;

	/// The type of the database. This is currently set to [`sqlx::Postgres`].
	/// A type alias is used here so that it can be referenced everywhere easily
	pub type DatabaseType = sqlx::Postgres;
}

use std::sync::OnceLock;

use tokio_util::sync::CancellationToken;

use crate::{prelude::*, utils::config::AppConfig};

/// The global cancellation token that will be used to cancel the connections
/// when the runner is stopped. This token will be used to cancel all the
/// connections that are open in the runner.
#[doc(hidden)]
static GLOBAL_CANCEL_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

/// Builds the application state from the config. This is used in the main
/// function to build the state that will be passed to the routes and other
/// functions.
pub async fn build_state(config: AppConfig) -> AppState {
	let database = db::connect(&config.database).await;

	let redis = redis::connect(&config.redis).await;

	let worker = worker::setup(&database);

	AppState {
		database,
		redis,
		config,
		worker,
	}
}

/// Returns a future that completes when the global cancellation token is
/// cancelled. Use this for graceful shutdown handlers.
pub async fn exit_signal() {
	GLOBAL_CANCEL_TOKEN
		.get_or_init(CancellationToken::new)
		.cancelled()
		.await
}

/// Triggers the shutdown of the application by cancelling the global
/// cancellation token. This is called when a shutdown signal is received, and
/// it will cause all the connections to be cancelled and the application to
/// shut down gracefully.
pub fn trigger_shutdown() {
	GLOBAL_CANCEL_TOKEN
		.get_or_init(CancellationToken::new)
		.cancel();
}
