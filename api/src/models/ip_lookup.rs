use std::net::IpAddr;

use ipinfo::{IpDetails, IpInfo};
use rustis::{client::Client as RedisClient, commands::StringCommands};

use crate::{prelude::*, utils::config::IpInfoConfig};

/// Performs an IP lookup using the ipinfo crate, with caching in Redis to
/// improve performance for repeated lookups. The function first checks if the
/// IP details are present in Redis, and if not, it performs the lookup using
/// ipinfo and stores the result in Redis for future use.
///
/// The results are cached in Redis using a key generated from the IP address
/// both to reduce latency and cost.
pub async fn lookup(
	ip: IpAddr,
	redis: &mut RedisClient,
	config: &IpInfoConfig,
) -> Result<IpDetails, ErrorType> {
	// First check if the IP lookup data is present in Redis
	let key = redis::keys::ip_lookup_data(ip);

	if let Some(Ok(cached_data)) = redis
		.get::<Option<String>>(&key)
		.await?
		.as_deref()
		.map(serde_json::from_str::<IpDetails>)
	{
		return Ok(cached_data);
	}

	// If not present in Redis, perform the IP lookup using the ipinfo crate
	let ip_details = IpInfo::new(ipinfo::IpInfoConfig {
		// If debug AND the token is empty, then don't use a token for testing. Otherwise use the
		// token.
		token: if cfg!(debug_assertions) && config.token.is_empty() {
			None
		} else {
			Some(config.token.clone())
		},
		..Default::default()
	})
	.inspect_err(|err| {
		info!("Error creating IpInfo: {err}");
	})?
	.lookup(ip.to_string().as_str())
	.await
	.inspect_err(|err| {
		info!("Error looking up IP address: {err}");
	})?;

	trace!("IP lookup successful: {:?}", ip_details);

	// Store the result in Redis for future lookups
	redis
		.setex(
			&key,
			if ip_details.bogon.unwrap_or(false) {
				// If the IP is a bogon, cache it for a shorter duration since it's likely to
				// change
				constants::IP_LOOKUP_FAILURE_VALIDITY
					.whole_seconds()
					.unsigned_abs()
			} else {
				constants::IP_LOOKUP_SUCCESS_VALIDITY
					.whole_seconds()
					.unsigned_abs()
			},
			serde_json::to_string(&ip_details)?,
		)
		.await?;

	Ok(ip_details)
}
