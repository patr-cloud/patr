use sqlx::{pool::PoolOptions, Pool};

use crate::{prelude::*, utils::config::DatabaseConfig};

pub(super) mod initializer;
pub(super) mod meta_data;
pub(super) mod rbac;
pub(super) mod user;
pub(super) mod workspace;

pub use self::initializer::initialize;
pub(super) use self::{meta_data::*, rbac::*, user::*, workspace::*};

/// Connects to the database based on a config. Not much to say here.
#[instrument(skip(config))]
pub async fn connect(config: &DatabaseConfig) -> Pool<DatabaseType> {
	info!("Connecting to database `{}:{}`", config.host, config.port);
	PoolOptions::<DatabaseType>::new()
		.max_connections(config.connection_limit.as_u64().unwrap() as u32)
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
