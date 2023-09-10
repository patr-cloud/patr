use rustis::client::Client;

use crate::utils::config::RedisConfig;

/// A list of all the keys to store data in Redis
pub mod keys;

/// Connect to a Redis server using the given configuration
#[tracing::instrument(skip(config))]
pub async fn connect(config: &RedisConfig) -> Client {
	Client::connect(format!(
		"{}://{}{}:{}/{}",
		if config.secure { "rediss" } else { "redis" },
		if let Some((username, password)) = config.user.as_ref().zip(config.password.as_ref()) {
			format!("{}:{}@", username, password)
		} else {
			"".to_string()
		},
		config.host,
		config.port,
		config.database
	))
	.await
	.expect("Failed to connect to Redis")
}
