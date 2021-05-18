mod initializer;
mod meta_data;
mod organisation;
mod rbac;
mod user;

pub use initializer::initialize;
pub use meta_data::*;
pub use organisation::*;
pub use rbac::*;
use redis::{aio::MultiplexedConnection, Client, RedisError};
use sqlx::{pool::PoolOptions, Connection, Database as Db, Pool};
use tokio::task;
pub use user::*;

use crate::{utils::settings::Settings, Database};

pub async fn create_database_connection(
	config: &Settings,
) -> Result<Pool<Database>, sqlx::Error> {
	log::trace!("Creating database connection pool...");
	PoolOptions::<Database>::new()
		.max_connections(config.database.connection_limit)
		.connect_with(
			<<Database as Db>::Connection as Connection>::Options::new()
				.username(&config.database.user)
				.password(&config.database.password)
				.host(&config.database.host)
				.port(config.database.port)
				.database(&config.database.database),
		)
		.await
}

pub async fn create_redis_connection(
	config: &Settings,
) -> Result<MultiplexedConnection, RedisError> {
	let (redis, redis_poller) = Client::open(format!(
		"redis://{}{}{}:{}/0",
		if let Some(user) = &config.redis.user {
			user
		} else {
			""
		},
		if let Some(password) = &config.redis.password {
			format!(":{}@", password)
		} else {
			"".to_string()
		},
		config.redis.host,
		config.redis.port
	))?
	.create_multiplexed_tokio_connection()
	.await?;
	task::spawn(redis_poller);

	Ok(redis)
}
