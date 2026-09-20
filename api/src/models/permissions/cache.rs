use rustis::{client::Client as RedisClient, commands::StringCommands as _};
use time::OffsetDateTime;

use crate::prelude::*;

/// Stamp one login's cached permissions as stale. For changes to the
/// credential itself: revoked, regenerated, updated, logged out.
pub async fn mark_login_stale(redis: &mut RedisClient, login_id: &Uuid) -> Result<(), ErrorType> {
	mark_stale_in_redis(redis, redis::keys::login_cache_stale_since(login_id)).await
}

/// Stamp every login of one actor (user or service account) as stale. For
/// changes every login inherits: password, MFA, roles, workspace membership.
pub async fn mark_actor_stale(redis: &mut RedisClient, actor_id: &Uuid) -> Result<(), ErrorType> {
	mark_stale_in_redis(redis, redis::keys::actor_cache_stale_since(actor_id)).await
}

/// Stamp every cached permission set on one workspace as stale. For role
/// changes and workspace deletion.
pub async fn mark_workspace_stale(
	redis: &mut RedisClient,
	workspace_id: &Uuid,
) -> Result<(), ErrorType> {
	mark_stale_in_redis(
		redis,
		redis::keys::workspace_cache_stale_since(workspace_id),
	)
	.await
}

/// Stamp everything as stale.
pub async fn mark_all_stale(redis: &mut RedisClient) -> Result<(), ErrorType> {
	mark_stale_in_redis(redis, redis::keys::all_cache_stale_since()).await
}

/// Write the current instant (unix nanos) to the stamp `key` with the cache TTL.
async fn mark_stale_in_redis(redis: &mut RedisClient, key: String) -> Result<(), ErrorType> {
	redis
		.setex(
			&key,
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| error!("Error setting the stale-since stamp `{key}`: `{err}`"))?;
	Ok(())
}
