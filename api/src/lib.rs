#![feature(impl_trait_in_assoc_type)]

//! The main API server for Patr.

#[cfg(not(target_arch = "wasm32"))]
/// This module contains the main application logic. Most of the app requests,
/// states, and mounting of endpoints are done here
pub mod app;
#[cfg(not(target_arch = "wasm32"))]
/// This module contains the database connection logic, as well as all the
/// ORM entities.
pub mod db;
#[cfg(not(target_arch = "wasm32"))]
/// This module contains the models used by the API. These are the structs that
/// are used for encoding and decoding things that are not a part of the API
/// (eg, JWT).
pub mod models;
#[cfg(not(target_arch = "wasm32"))]
/// This module contains the Redis connection and all utilities to set and get
/// data in Redis.
pub mod redis;
#[cfg(not(target_arch = "wasm32"))]
/// This module is used to listen for changes in the database and publish them
/// to Redis. This is used for the real-time updates on stream requests.
pub mod redis_publisher;
#[cfg(not(target_arch = "wasm32"))]
/// This module contains the routes for the API. This is where the endpoints
/// are mounted.
pub mod routes;
#[cfg(not(target_arch = "wasm32"))]
/// This module contains all the utilities used by the API. This includes things
/// like the config parser, the [`tower::Layer`]s that are used to parse the
/// requests.
pub mod utils;

#[cfg(not(target_arch = "wasm32"))]
/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use macros::query;
	pub use models::{
		ApiEndpoint,
		AppResponse,
		ErrorType,
		api::WithId,
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
		utils::{OneOrMore, Paginated, Uuid},
	};
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{
			AppRequest,
			AppState,
			AuthenticatedAppRequest,
			ProcessedApiRequest,
			UnprocessedAppRequest,
		},
		redis,
		utils::{RouterExt, TimeoutExt, constants},
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

#[cfg(not(target_arch = "wasm32"))]
/// Listen for the exit signal and stop the server when the signal is received.
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
}
