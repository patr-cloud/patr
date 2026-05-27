use std::{
	convert::Infallible,
	net::{IpAddr, SocketAddr},
	sync::LazyLock,
};

use axum::{
	extract::{ConnectInfo, FromRequestParts},
	http::request::Parts,
};
use sqlx::types::ipnetwork::IpNetwork;
use tracing::warn;

/// Extractor for the client IP address.
///
/// In production the TCP peer is always nginx (loopback or a private bridge
/// IP). When the peer is in a private range we trust `X-Real-IP` written by
/// nginx; otherwise we ignore forwarded headers and fall back to the socket
/// peer, emitting a warning since the API socket should never be reached
/// directly from outside the trust boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientIP(
	/// The IP address of the client.
	pub IpAddr,
);

impl FromRequestParts<()> for ClientIP {
	type Rejection = Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &()) -> Result<Self, Self::Rejection> {
		static PRIVATE_RANGES: LazyLock<Vec<IpNetwork>> = LazyLock::new(|| {
			[
				"127.0.0.0/8",    // IPv4 loopback
				"::1/128",        // IPv6 loopback
				"10.0.0.0/8",     // RFC1918 private
				"172.16.0.0/12",  // RFC1918 private
				"192.168.0.0/16", // RFC1918 private
				"fc00::/7",       // IPv6 ULA (RFC4193)
				"169.254.0.0/16", // IPv4 link-local (RFC3927)
				"fe80::/10",      // IPv6 link-local
				"100.64.0.0/10",  /* CGNAT / shared address space (RFC6598) — k8s pod nets on
				                   * some EKS configs */
			]
			.iter()
			.map(|s| s.parse().expect("static CIDR literal"))
			.collect()
		});

		let peer = ConnectInfo::<SocketAddr>::from_request_parts(parts, &())
			.await
			.unwrap()
			.ip();

		if !PRIVATE_RANGES.iter().any(|net| net.contains(peer)) {
			warn!(
				?peer,
				"API socket reached from non-private peer; ignoring forwarded headers"
			);
			return Ok(Self(peer));
		}

		let ip = parts
			.headers
			.get("X-Real-IP")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse().ok())
			.unwrap_or(peer);

		Ok(Self(ip))
	}
}
