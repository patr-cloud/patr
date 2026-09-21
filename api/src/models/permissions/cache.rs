//! The Redis cache of authenticated actors, keyed by login ID.
//!
//! One entry per login ([`ActorAuthDataCache`]) holds everything a request
//! needs, so a hit costs no database round trip. Entries are never updated in
//! place: anything that changes what an entry would contain bumps a
//! `*_cache_stale_since` stamp for its scope instead, and the next read of an
//! entry older than a stamp covering it is a miss.
//!
//! Stamps live exactly as long as entries do
//! ([`constants::CACHED_PERMISSIONS_VALIDITY`]), which is sufficient since a
//! stamp only has to outlive the entries created before it. That holds only
//! while Redis never evicts: with `maxmemory-policy noeviction` (the default)
//! a stamp can't disappear before the entries it covers. Don't run this cache
//! on an evicting Redis.

use rustis::{
	client::Client as RedisClient,
	commands::{GenericCommands as _, StringCommands as _},
};
use time::OffsetDateTime;

use crate::{models::redis::ActorAuthDataCache, prelude::*};

/// Read the cached entry for `login_id`, if there is one and no stamp covering
/// it is newer than it. Two round trips: the entry, then — since the entry
/// says which actor and workspaces it spans — every stamp that covers it.
///
/// A Redis error or an undecodable entry is a miss, not an error: the caller
/// falls back to the database and the next request tries again.
pub(super) async fn read(redis: &mut RedisClient, login_id: &Uuid) -> Option<ActorAuthDataCache> {
	let entry_key = redis::keys::auth_data_for_login_id(login_id);

	let entry = redis
		.get::<Option<String>>(&entry_key)
		.await
		.inspect_err(|err| {
			error!("Error reading the cached auth data for login `{login_id}`: `{err}`")
		})
		.ok()??;
	let entry = serde_json::from_str::<ActorAuthDataCache>(&entry)
		.inspect_err(|err| {
			warn!("Discarding an undecodable cache entry for login `{login_id}`: `{err}`");
		})
		.ok()?;

	let stale = redis
		.mget::<Vec<Option<String>>>(
			[
				redis::keys::login_cache_stale_since(login_id),
				redis::keys::all_cache_stale_since(),
				redis::keys::actor_cache_stale_since(&entry.actor_id),
			]
			.into_iter()
			.chain(
				entry
					.permissions
					.keys()
					.map(redis::keys::workspace_cache_stale_since),
			)
			.collect::<Vec<_>>(),
		)
		.await
		.inspect_err(|err| {
			error!("Error reading the stale-since stamps for login `{login_id}`: `{err}`")
		})
		.ok()?
		.into_iter()
		// Flatten all timestamps to get only the ones that actually exist (Option is an iterator)
		.flatten()
		.filter_map(|stamp| stamp.parse::<i128>().ok())
		.filter_map(|nanos| OffsetDateTime::from_unix_timestamp_nanos(nanos).ok())
		// If any of them is stale, the entire entry is stale
		.any(|stamp| entry.created_at < stamp);

	if stale {
		trace!("Cached auth data for login `{login_id}` is stale");
		// Tidy up so requests that keep failing to refetch (say, a revoked
		// token) don't keep paying for the stamp reads.
		_ = redis.del(&entry_key).await;
		return None;
	}

	Some(entry)
}

/// Cache `entry` under `login_id` for `ttl`. Overwrites whatever is there.
///
/// A Redis error is logged and dropped: the request proceeds uncached and the
/// next one writes again.
pub(super) async fn write(
	redis: &mut RedisClient,
	login_id: &Uuid,
	entry: &ActorAuthDataCache,
	ttl: time::Duration,
) {
	let Ok(value) = serde_json::to_string(entry).inspect_err(|err| {
		error!("Error serialising the auth data for login `{login_id}`: `{err}`")
	}) else {
		return;
	};

	_ = redis
		.setex(
			redis::keys::auth_data_for_login_id(login_id),
			ttl.whole_seconds().unsigned_abs(),
			value,
		)
		.await
		.inspect_err(|err| error!("Error caching the auth data for login `{login_id}`: `{err}`"));
}

/// Stamp one login's cached entry as stale, and drop the entry itself. For
/// changes to the credential: revoked, regenerated, updated, logged out.
///
/// This is the one scope whose entry key is known, so the entry is deleted
/// outright and revocation doesn't rest on the stamp alone. The stamp still
/// matters for a request that missed the cache before the change and writes
/// its (now stale) entry after it.
pub async fn mark_login_stale(redis: &mut RedisClient, login_id: &Uuid) -> Result<(), ErrorType> {
	mark_stale_in_redis(redis, redis::keys::login_cache_stale_since(login_id)).await?;
	redis
		.del(redis::keys::auth_data_for_login_id(login_id))
		.await
		.inspect_err(|err| {
			error!("Error deleting the cached auth data for login `{login_id}`: `{err}`")
		})?;
	Ok(())
}

/// Stamp every login of one actor (user or service account) as stale. For
/// changes every login inherits: password, MFA, roles, workspace membership.
pub async fn mark_actor_stale(redis: &mut RedisClient, actor_id: &Uuid) -> Result<(), ErrorType> {
	mark_stale_in_redis(redis, redis::keys::actor_cache_stale_since(actor_id)).await
}

/// Stamp every cached entry holding permissions on one workspace as stale.
/// For role changes and workspace deletion.
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
