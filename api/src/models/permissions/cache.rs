//! The Redis cache of authenticated actors, keyed by login ID for web logins
//! and by token hash for opaque tokens.
//!
//! One entry per credential ([`ActorAuthDataCache`]) holds everything a request
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

/// Read the cached entry at `key`, if there is one and no stamp covering it
/// is newer than it. Two round trips: the entry, then — since the entry says
/// which login, actor and workspaces it spans — every stamp that covers it.
///
/// A Redis error or an undecodable entry is a miss, not an error: the caller
/// falls back to the database and the next request tries again.
pub(super) async fn read(redis: &mut RedisClient, key: &str) -> Option<ActorAuthDataCache> {
	let entry = redis
		.get::<Option<String>>(key)
		.await
		.inspect_err(|err| error!("Error reading cached auth data: `{err}`"))
		.ok()??;
	let entry = serde_json::from_str::<ActorAuthDataCache>(&entry)
		.inspect_err(|err| warn!("Discarding an undecodable auth data entry: `{err}`"))
		.ok()?;

	let stale = redis
		.mget::<Vec<Option<String>>>(
			[
				redis::keys::login_cache_stale_since(&entry.login_id),
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
			error!(
				"Error reading the stale-since stamps for login `{}`: `{err}`",
				entry.login_id
			)
		})
		.ok()?
		.into_iter()
		.flatten()
		.filter_map(|stamp| stamp.parse::<i128>().ok())
		.filter_map(|nanos| OffsetDateTime::from_unix_timestamp_nanos(nanos).ok())
		.any(|stamp| entry.created_at < stamp);

	if stale {
		trace!("Cached auth data for login `{}` is stale", entry.login_id);
		// Tidy up so requests that keep failing to refetch (say, a revoked
		// token) don't keep paying for the stamp reads.
		_ = redis.del(key).await;
		return None;
	}

	Some(entry)
}

/// Cache `entry` at `key` for `ttl`. Overwrites whatever is there.
///
/// A Redis error is logged and dropped: the request proceeds uncached and the
/// next one writes again.
pub(super) async fn write(
	redis: &mut RedisClient,
	key: &str,
	entry: &ActorAuthDataCache,
	ttl: time::Duration,
) {
	let Ok(value) = serde_json::to_string(entry).inspect_err(|err| {
		error!(
			"Error serialising the auth data for login `{}`: `{err}`",
			entry.login_id
		)
	}) else {
		return;
	};

	_ = redis
		.setex(key, ttl.whole_seconds().unsigned_abs(), value)
		.await
		.inspect_err(|err| {
			error!(
				"Error caching the auth data for login `{}`: `{err}`",
				entry.login_id
			)
		});
}

/// Stamp one web login's cached entry as stale, and drop the entry itself. For
/// a logout or a deleted web login.
///
/// This is a scope whose entry key is known, so the entry is deleted
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

/// Stamp an opaque token's login as stale, and drop the token's cached
/// entry. For API tokens and service account tokens, whose entries are keyed
/// by the token's hash rather than the login ID: the caller reads that hash
/// back from the row it changed (the old one, when the token is regenerated).
pub async fn mark_token_stale(
	redis: &mut RedisClient,
	login_id: &Uuid,
	token_hash: &str,
) -> Result<(), ErrorType> {
	mark_stale_in_redis(redis, redis::keys::login_cache_stale_since(login_id)).await?;
	redis
		.del(redis::keys::auth_data_for_token(token_hash))
		.await
		.inspect_err(|err| {
			error!("Error deleting the cached auth data for token login `{login_id}`: `{err}`")
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
