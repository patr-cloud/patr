#[allow(dead_code)]
mod rbac;

use redis::{
	aio::MultiplexedConnection as RedisConnection,
	Client,
	RedisError,
};
use tokio::task;

pub use self::rbac::*;
use crate::utils::settings::Settings;

pub async fn create_redis_connection(
	config: &Settings,
) -> Result<RedisConnection, RedisError> {
	log::trace!("Creating redis connection pool...");

	let schema = if config.redis.secure {
		"rediss"
	} else {
		"redis"
	};
	let username = config.redis.user.as_deref().unwrap_or("");
	let password = config
		.redis
		.password
		.as_deref()
		.map_or_else(|| "".to_string(), |pwd| format!(":{}@", pwd));
	let host = config.redis.host.as_str();
	let port = config.redis.port;
	let database = config.redis.database.unwrap_or(0);

	let (redis, redis_poller) = Client::open(format!(
		"{schema}://{username}{password}{host}:{port}/{database}"
	))?
	.create_multiplexed_tokio_connection()
	.await?;
	task::spawn(redis_poller);

	Ok(redis)
}
