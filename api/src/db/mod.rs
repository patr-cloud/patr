mod initializer;
mod meta_data;
mod organisation;
mod rbac;
mod user;

use crate::utils::settings::Settings;
use redis::{aio::MultiplexedConnection, Client, RedisError};
use sqlx::{
	mysql::{MySqlConnectOptions, MySqlPoolOptions},
	MySqlPool,
};
use tokio::task;

pub use initializer::initialize;
pub use meta_data::*;
pub use organisation::*;
pub use rbac::*;
pub use user::*;

pub async fn create_mysql_connection(
	config: &Settings,
) -> Result<MySqlPool, sqlx::Error> {
	log::trace!("Creating database connection pool...");
	MySqlPoolOptions::new()
		.max_connections(config.mysql.connection_limit)
		.connect_with(
			MySqlConnectOptions::new()
				.username(&config.mysql.user)
				.password(&config.mysql.password)
				.host(&config.mysql.host)
				.port(config.mysql.port)
				.database(&config.mysql.database),
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
	.create_multiplexed_async_std_connection()
	.await?;
	task::spawn(redis_poller);

	Ok(redis)
}
