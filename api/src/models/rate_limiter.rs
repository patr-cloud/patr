use std::{
	net::IpAddr,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use rustis::{
	client::{BatchPreparedCommand as _, Client as RedisClient},
	commands::{GenericCommands as _, SortedSetCommands as _, ZAddOptions},
};

use crate::{prelude::*, redis::keys};

/// Checks rate limits for a given identifier using the **sliding window log**
/// algorithm with Redis sorted sets, based on the approach described at
/// <https://www.peakscale.com/redis-rate-limiting/>.
///
/// ## How it works
///
/// A Redis **sorted set** is a collection of unique string members, each with a
/// floating-point score. Members are ordered by score, and Redis provides
/// efficient commands to add entries, remove entries by score range, and count
/// the total number of entries. This makes sorted sets ideal for tracking
/// timestamped events within a sliding window.
///
/// For each rate limit window (e.g. 3 requests per second), we maintain one
/// sorted set keyed by `rateLimit:{ip|loginId}:{identifier}:{window_secs}`.
/// Each entry in the set represents one request:
///
/// - **Score**: the request timestamp in nanoseconds (as `f64`)
/// - **Member**: `"{key}-{timestamp}"` — unique per identifier per nanosecond
///
/// ## Pipeline commands (per window)
///
/// All commands for all windows are batched into a single Redis pipeline (one
/// round trip):
///
/// 1. **`ZADD key score member`** — optimistically record this request in the sorted set, *before*
///    checking if the limit is exceeded. This ensures rejected requests still count against the
///    limit, preventing attackers from retrying without penalty.
/// 2. **`ZREMRANGEBYSCORE key -inf (now - window)`** — prune all entries whose timestamp falls
///    outside the current sliding window.
/// 3. **`ZCARD key`** — count the remaining entries. If this exceeds the limit, the request is
///    rejected.
/// 4. **`EXPIRE key (window_secs + 10)`** — refresh the key's TTL so it auto-expires if no further
///    requests arrive. The 10-second buffer prevents premature expiry at window boundaries.
///
/// ## Scope
///
/// When `login_id` is `Some`, only the per-login bucket is checked.
/// Authenticated callers are governed by their login: the per-login bucket is
/// the abuse signal that matters, and a shared per-IP bucket would unfairly
/// penalise users behind NAT/CGNAT who share a public IP with other tenants.
/// When `None`, only the per-IP bucket is checked (unauthenticated endpoints).
pub async fn check_rate_limit(
	redis: &mut RedisClient,
	client_ip: IpAddr,
	login_id: Option<&Uuid>,
	limits: &[(u32, Duration)],
) -> Result<(), ErrorType> {
	// f64 score has ~256ns precision loss at this scale, which is negligible
	// for rate limiting windows of 1s/60s/3600s.
	let now_ns = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("system clock is before UNIX epoch")
		.as_nanos() as f64;

	if let Some(login_id) = login_id {
		check_single_rate_limit(redis, now_ns, limits, |window_secs| {
			keys::rate_limit_login_id(login_id, window_secs)
		})
		.await?;
	} else {
		let ip_key = match client_ip {
			// IPv4: use full address.
			IpAddr::V4(v4) => v4.to_string(),

			// IPv6: mask to /64 subnet since it's easy to
			// obtain many addresses within a /64 allocation.
			IpAddr::V6(v6) => {
				let [first, second, third, fourth, ..] = v6.segments();
				format!("{first:x}:{second:x}:{third:x}:{fourth:x}::")
			}
		};

		check_single_rate_limit(redis, now_ns, limits, |window_secs| {
			keys::rate_limit_ip(&ip_key, window_secs)
		})
		.await?;
	}

	Ok(())
}

/// Runs the sliding window log check against a single set of keys.
async fn check_single_rate_limit(
	redis: &mut RedisClient,
	now_ns: f64,
	limits: &[(u32, Duration)],
	key_fn: impl Fn(u64) -> String,
) -> Result<(), ErrorType> {
	let mut pipeline = redis.create_pipeline();

	for &(_, duration) in limits {
		let key = key_fn(duration.as_secs());
		let window_ns = duration.as_nanos() as f64;

		(&mut pipeline)
			.zadd(&key, (now_ns, now_ns.to_string()), ZAddOptions::default())
			.forget();
		(&mut pipeline)
			.zremrangebyscore(&key, f64::NEG_INFINITY, now_ns - window_ns)
			.forget();
		(&mut pipeline).zcard(&key).queue();
		(&mut pipeline)
			.expire(&key, duration.as_secs() + 10, None)
			.forget();
	}

	let counts = pipeline
		.execute::<Vec<usize>>()
		.await
		.map_err(ErrorType::server_error)?;

	for (&(max_count, _), &count) in limits.iter().zip(counts.iter()) {
		if count > max_count as usize {
			return Err(ErrorType::RateLimitExceeded);
		}
	}

	Ok(())
}
