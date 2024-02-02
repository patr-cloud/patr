mod auth;

use redis::{
	aio::MultiplexedConnection as RedisConnection,
	Client,
	RedisError,
};

pub use self::auth::*;
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

	let redis = Client::open(format!(
		"{schema}://{username}{password}{host}:{port}/{database}"
	))?
	.get_multiplexed_tokio_connection()
	.await?;

	Ok(redis)
}
