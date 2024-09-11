#![feature(impl_trait_in_assoc_type)]

//! Common utilities for the runner. This library contains all the things you
//! will need to make a runner. All it needs are the implementations of how the
//! runner handles the resources (such as deployments, databases, etc).

/// This module contains the main application logic. Most of the app requests,
/// states, and mounting of endpoints are done here
mod app;
/// The database module contains all the database related functions. Such as
/// initializing the database, getting the connection, etc.
mod db;
/// The executor module contains the trait that the runner needs to implement
/// to run the resources.
mod executor;
/// All the Routes for the self-hosted Patr is defined here.
mod routes;
/// All the runner related structs and functions.
mod runner;
/// This module contains all the utilities used by the API. This includes things
/// like the config parser, the [`tower::Layer`]s that are used to parse the
/// requests, etc.
mod utils;

/// The prelude module contains all the things you need to import to get
/// started with the runner.
pub mod prelude {
	pub use macros::version;
	pub use models::prelude::*;
	pub use sqlx::{query, Row};

	pub use crate::{
		app::{AppRequest, AppState, ProcessedApiRequest},
		executor::RunnerExecutor,
		runner::Runner,
		utils::{client, config::*, constants, ext_traits::*},
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
