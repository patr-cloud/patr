#[path = "api.patr.cloud/mod.rs"]
mod api_patr_cloud;

use std::net::{IpAddr, SocketAddr};

use api_models::ErrorType;
use async_trait::async_trait;
use axum::{
	extract::{ConnectInfo, FromRequestParts},
	http::request::Parts,
	Router,
};

use crate::app::App;

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new().merge(api_patr_cloud::create_sub_app(app))
}

#[derive(Debug)]
pub struct ClientIp(pub IpAddr);

#[async_trait]
impl<S> FromRequestParts<S> for ClientIp
where
	S: Sync,
{
	// todo: use custom error msg if no valid ip address is found
	type Rejection = api_models::ErrorType;

	async fn from_request_parts(
		parts: &mut Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		{
			// prefer to extract Cloudflare provided IP
			parts
				.headers
				.get("CF-Connecting-IP")
				.and_then(|hv| hv.to_str().ok())
				.and_then(|s| s.parse::<IpAddr>().ok())
		}
		.or_else(|| {
			parts
				.headers
				.get("X-Real-IP")
				.and_then(|hv| hv.to_str().ok())
				.and_then(|s| s.parse::<IpAddr>().ok())
		})
		.or_else(|| {
			// use the first valid IP from X-Forwarded-For IP list
			parts
				.headers
				.get_all("X-Forwarded-For")
				.iter()
				.filter_map(|hv| hv.to_str().ok())
				.filter_map(|s| s.parse::<IpAddr>().ok())
				.next()
		})
		.or_else(|| {
			// use connected IP to server as fallback
			parts
				.extensions
				.get::<ConnectInfo<SocketAddr>>()
				.map(|ConnectInfo(addr)| addr.ip())
		})
		.map(Self)
		.ok_or_else(|| ErrorType::internal_error())
	}
}
