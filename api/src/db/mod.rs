use sqlx::{pool::PoolOptions, Pool};

use crate::{prelude::*, utils::config::DatabaseConfig};

/// The initializer for the database. This will create the database pool and
/// initialize the database with the necessary tables and data.
pub(super) mod initializer;
/// The meta data for the database. This is mostly used for the version number
/// of the database and handling the migrations for the database.
pub(super) mod meta_data;
/// The role based access control for the database. This is used to handle the
/// permissions for the users and what workspace they have access to.
pub(super) mod rbac;
/// The user module for the database. This is used to handle the users and their
/// data.
pub(super) mod user;
/// The workspace module for the database. This is used to handle the workspaces
/// and their data.
pub(super) mod workspace;

pub use self::initializer::initialize;
pub(super) use self::{meta_data::*, rbac::*, user::*, workspace::*};

/// Connects to the database based on a config. Not much to say here.
#[instrument(skip(config))]
pub async fn connect(config: &DatabaseConfig) -> Pool<DatabaseType> {
	info!("Connecting to database `{}:{}`", config.host, config.port);
	PoolOptions::<DatabaseType>::new()
		.max_connections(config.connection_limit)
		.connect_with(
			<DatabaseConnection as sqlx::Connection>::Options::new()
				.username(config.user.as_str())
				.password(config.password.as_str())
				.host(config.host.as_str())
				.port(config.port)
				.database(config.database.as_str()),
		)
		.await
		.expect("Failed to connect to database")
}
