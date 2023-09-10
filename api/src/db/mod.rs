use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::utils::config::DatabaseConfig;

#[allow(missing_docs)]
mod entities;

pub use self::entities::{prelude::*, *};

/// Connects to the database based on a config. Not much to say here.
#[tracing::instrument(skip(config))]
pub async fn connect(config: &DatabaseConfig) -> DatabaseConnection {
	let mut connect_options = ConnectOptions::new(format!(
		"postgres://{}:{}@{}:{}/{}",
		config.user, config.password, config.host, config.port, config.database
	));
	connect_options.max_connections(config.connection_limit);
	Database::connect(connect_options)
		.await
		.expect("Failed to connect to database")
}
