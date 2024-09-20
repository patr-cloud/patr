use sqlx::{pool::PoolOptions, Pool};

use crate::prelude::*;

/// The initializer for the database. This will create the database pool and
/// initialize the database with the necessary tables and data.
pub(super) mod initializer;
/// The meta data for the database. This is mostly used for the version number
/// of the database and handling the migrations for the database.
pub(super) mod meta_data;
/// The workspace module for the database. This is used to handle the workspaces
/// and their data.
pub(super) mod workspace;

pub(super) use self::{initializer::initialize, meta_data::*, workspace::*};

/// Connects to the database based on a config. Not much to say here.
#[instrument(skip(config))]
pub async fn connect(config: &DatabaseConfig) -> Pool<DatabaseType> {
	info!("Connecting to database: `{}`", config.file);
	PoolOptions::<DatabaseType>::new()
		.max_connections(config.connection_limit)
		.connect_with(
			<DatabaseConnection as sqlx::Connection>::Options::new()
				.filename(&config.file)
				.foreign_keys(true)
				.create_if_missing(true),
		)
		.await
		.expect("Failed to connect to database")
}
