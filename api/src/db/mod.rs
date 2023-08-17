use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::utils::config::DatabaseConfig;

mod entities;

pub use self::entities::prelude::*;

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
