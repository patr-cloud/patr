//! Common utilities for the runner. This library contains all the things you
//! will need to make a runner. All it needs are the implementations of how the
//! runner handles the resources (such as deployments, databases, etc).

/// The database module contains all the database related functions. Such as
/// initializing the database, getting the connection, etc.
pub mod db;
/// The error module contains all the error types that are used in the runner.
/// This includes the error type for the runner, the error type for the
/// database, etc.
pub mod error;
/// The executor module contains the trait that the runner needs to implement
/// to run the resources.
pub mod executor;
/// Database migrations organized by version. Uses `inventory` for automatic
/// registration and tracks individual migrations in a `migrations` table.
pub mod migrations;
/// All the runner related structs and functions: the runner itself, and the
/// controller, observer and WebSocket loops it runs.
pub mod runner;
/// This module contains all the utilities used by the runner. This includes
/// things like the config parser, the upstream API client, etc.
pub mod utils;

/// The prelude module contains all the things you need to import to get
/// started with the runner.
pub mod prelude {
	pub use macros::{query, version};
	pub use models::{api::workspace::runner::RunnerExposureType, prelude::*};
	pub use sqlx::Row;

	pub use crate::{
		error::RunnerError,
		executor::{
			DeleteOutcome,
			ExpectedGraph,
			FinalizerKind,
			LocalManagedUrl,
			LocalState,
			Object,
			Owner,
			RunnerExecutor,
		},
		runner::Runner,
		utils::{client, config::*, constants, ext_traits::*, secrets},
	};

	/// The type of the database connection. A mutable reference to this should
	/// be used as the parameter for database functions, since it accepts both a
	/// connection and a transaction.
	///
	/// Example:
	/// ```rust
	/// # use common::prelude::*;
	/// pub fn database_fn(connection: &mut DatabaseConnection) {
	///     // Do something with `connection` ....
	/// }
	/// ```
	pub type DatabaseConnection = <DatabaseType as sqlx::Database>::Connection;

	/// The type of the database transaction.
	///
	/// This is used in requests to rollback or commit transactions based on how
	/// an endpoint responds. This currently has a static lifetime, implying
	/// that only transactions from a pooled connection is allowed.
	pub type DatabaseTransaction = sqlx::Transaction<'static, DatabaseType>;

	/// The type of the database. This is currently set to [`sqlx::Sqlite`].
	/// A type alias is used here so that it can be referenced everywhere easily
	pub type DatabaseType = sqlx::Sqlite;
}
