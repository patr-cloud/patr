use std::{
	net::{IpAddr, SocketAddr},
	str::FromStr,
};

use axum::{
	extract::{ConnectInfo, FromRequestParts},
	http::request::Parts,
};
use models::ErrorType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientIP(pub IpAddr);

#[axum::async_trait]
impl<S> FromRequestParts<S> for ClientIP {
	type Rejection = ErrorType;

	async fn from_request_parts(
		parts: &mut Parts,
		_: &S,
	) -> Result<Self, Self::Rejection> {
		let cf_connecting_ip = parts
			.headers
			.get("CF-Connecting-IP")
			.and_then(|header_value| header_value.to_str().ok())
			.and_then(|value| IpAddr::from_str(value).ok());
		let x_real_ip = parts
			.headers
			.get("X-Real-IP")
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

		Ok(Self(
			cf_connecting_ip
				.or(x_real_ip)
				.or(x_forwarded_for)
				.unwrap_or(ip),
		))
	}
}
