// TODO:
// - Figure out SQLX migrations
// - Figure out slqx queries, and make them not throw errors and give proper
//   type
// - Figure out sqlite constraints / enums and all
// - Figure out how to run all this and test it.

#![feature(impl_trait_in_assoc_type, async_closure)]

//! Common utilities for the runner. This library contains all the things you
//! will need to make a runner. All it needs are the implementations of how the
//! runner handles the resources (such as deployments, databases, etc).

/// This module contains the main application logic. Most of the app requests,
/// states, and mounting of endpoints are done here
pub mod app;
/// The client for the Patr API to get runner data for a given workspace.
mod client;
/// The configuration for the runner.
mod config;
/// The database module contains all the database related functions. Such as
/// initializing the database, getting the connection, etc.
mod db;
/// A utility that returns a value after a delay.
mod delayed_future;
/// The executor module contains the trait that the runner needs to implement
/// to run the resources.
mod executor;
/// Extensions traits for the `Either` type.
mod ext_traits;
/// All the Routes for the self-hosted Patr is defined here.
mod routes;
/// All the runner related structs and functions.
mod runner;

/// The constants module contains all the constants that are used throughout
/// the runner Project.
pub mod constants {
	use semver::Version;

	pub const SQLITE_DATABASE_PATH: &str = "db.db";

	pub const DATABASE_VERSION: Version = macros::version!();
}

/// The prelude module contains all the things you need to import to get
/// started with the runner.
pub mod prelude {
	pub use macros::{query, version};
	pub use models::prelude::*;

	pub(crate) use crate::ext_traits::EitherExt;
	pub use crate::{app::AppState, config::*, executor::RunnerExecutor, runner::Runner};

	/// The client module to interface with the Patr API
	pub mod client {
		pub use crate::client::*;
	}

	pub use crate::constants;

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

	/// The type of the database. This is currently set to [`sqlx::Sqlite`].
	/// A type alias is used here so that it can be referenced everywhere easily
	pub type DatabaseType = sqlx::Sqlite;
}
