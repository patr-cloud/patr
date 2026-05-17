use std::net::IpAddr;

use crate::prelude::*;

cfg_if! {
	if #[cfg(feature = "cloud")] {
		/// Performs an IP lookup using the ipinfo crate, with caching in Redis
		/// to improve performance for repeated lookups. The function first
		/// checks if the IP details are present in Redis, and if not, it
		/// performs the lookup using ipinfo and stores the result in Redis for
		/// future use.
		pub async fn lookup(ip: IpAddr, state: &AppState) -> Result<ipinfo::IpDetails, ErrorType> {
			use ipinfo::IpInfo;
			use rustis::commands::StringCommands as _;

			// In tests / local dev with no ipinfo token, short-circuit with a
			// stub instead of hitting the public ipinfo API. With random
			// per-call client IPs the cache misses every call and the
			// public-tier rate limit kicks in within seconds.
			if cfg!(debug_assertions) && state.config.ipinfo.token.is_empty() {
				return Ok(ipinfo::IpDetails {
					ip: ip.to_string(),
					..Default::default()
				});
			}

			// First check if the IP lookup data is present in Redis
			let key = redis::keys::ip_lookup_data(ip);

			if let Some(Ok(cached_data)) = state
				.redis
				.get::<Option<String>>(&key)
				.await?
				.as_deref()
				.map(serde_json::from_str::<ipinfo::IpDetails>)
			{
				return Ok(cached_data);
			}

			// If not present in Redis, perform the IP lookup using the ipinfo
			// crate
			let ip_details = IpInfo::new(ipinfo::IpInfoConfig {
				token: Some(state.config.ipinfo.token.clone()),
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
			state
				.redis
				.setex(
					&key,
					if ip_details.bogon.unwrap_or(false) {
						// If the IP is a bogon, cache it for a shorter
						// duration since it's likely to change
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
	} else {
		/// Stub `IpDetails` for self-hosted: every field is empty / zero so
		/// callers can uniformly read it without geolocation. `loc` is
		/// `"0,0"` so the standard "lat,lng" parse path doesn't error.
		#[derive(Debug, Clone)]
		pub struct IpDetails {
			/// The IP address that was looked up.
			pub ip: String,
			/// Whether the IP is a bogon (private / reserved). Always `None`
			/// in self-hosted.
			pub bogon: Option<bool>,
			/// Latitude/longitude as `"lat,lng"`. Defaults to `"0,0"`.
			pub loc: String,
			/// ISO country code. Empty in self-hosted.
			pub country: String,
			/// Region name. Empty in self-hosted.
			pub region: String,
			/// City name. Empty in self-hosted.
			pub city: String,
			/// IANA timezone. `None` in self-hosted.
			pub timezone: Option<String>,
		}

		impl IpDetails {
			/// Builds an empty stub for the given IP.
			fn empty(ip: IpAddr) -> Self {
				Self {
					ip: ip.to_string(),
					bogon: None,
					loc: String::from("0,0"),
					country: String::new(),
					region: String::new(),
					city: String::new(),
					timezone: None,
				}
			}
		}

		/// Self-hosted stub: returns an empty `IpDetails` without contacting
		/// any external service.
		pub async fn lookup(ip: IpAddr, _state: &AppState) -> Result<IpDetails, ErrorType> {
			Ok(IpDetails::empty(ip))
		}
	}
}
