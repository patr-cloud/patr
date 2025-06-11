use std::{
	convert::Infallible,
	net::{IpAddr, SocketAddr},
	str::FromStr,
};

use axum::{
	extract::{ConnectInfo, FromRequestParts},
	http::request::Parts,
};

/// Extractor for client IP address, which tries to get the IP address from
/// Cloudflare headers first, then from X-Real-IP header, then from
/// X-Forwarded-For header, and finally from the socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientIP(
	/// The IP address of the client.
	pub IpAddr,
);

#[axum::async_trait]
impl FromRequestParts<()> for ClientIP {
	type Rejection = Infallible;

	async fn from_request_parts(parts: &mut Parts, _: &()) -> Result<Self, Self::Rejection> {
		let cf_connecting_ip = parts
			.headers
			.get("CF-Connecting-IP")
			.and_then(|header_value| header_value.to_str().ok())
			.and_then(|value| IpAddr::from_str(value).ok());
		let x_forwarded_for = parts
			.headers
			.get("X-Forwarded-For")
			.and_then(|header_value| header_value.to_str().ok())
			.and_then(|value| {
				value
					.split(',')
					.next()
					.and_then(|ip| IpAddr::from_str(ip.trim()).ok())
			});
		let ip = ConnectInfo::<SocketAddr>::from_request_parts(parts, &())
			.await
			.unwrap()
			.ip();

		Ok(Self(cf_connecting_ip.or(x_forwarded_for).unwrap_or(ip)))
	}
}
