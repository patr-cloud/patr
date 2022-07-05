use std::time::Duration;

use api_models::utils::Uuid;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use redis::{
	aio::MultiplexedConnection as RedisConnection,
	AsyncCommands,
	RedisError,
};
use tokio::time;

// acquire lock on resource so that only one person can update their resource
// count at a time
pub async fn acquire_lock_on_resource(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
	resource_type: &str,
) -> Result<String, RedisError> {
	// TODO: set a KV with ttl
	let value = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(16)
		.map(char::from)
		.collect::<String>();

	let mut tries = 0;

	loop {
		// create a lock on redis for the resource
		let lock = redis_conn
			.set_nx::<String, String, String>(
				format!("{}-{}", workspace_id, resource_type),
				value.clone(),
			)
			.await;

		// if there is some error then it is possible that it might be due to
		// resource being locked try 10 times before exiting the loop
		if let Err(redis_err) = lock {
			if tries > 10 {
				return Err(redis_err);
			}

			tries += 1;

			time::sleep(Duration::from_millis(1000)).await;

			continue;
		} else {
			break;
		}
	}

	Ok(value)
}

pub async fn delete_lock_on_resource(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
	resource_type: &str,
	value: String,
) -> Result<(), RedisError> {
	// check if the same key value pair exists, if they do then delete them,
	// else don't delete them
	if redis_conn
		.get::<String, String>(format!("{}-{}", workspace_id, resource_type))
		.await? == value
	{
		redis_conn
			.del(format!("{}-{}", workspace_id, resource_type))
			.await?;
	}

	Ok(())
}
