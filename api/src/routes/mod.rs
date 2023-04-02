#[path = "api.patr.cloud/mod.rs"]
mod api_patr_cloud;
#[path = "assets.patr.cloud/mod.rs"]
mod assets_patr_cloud;
#[path = "auth.patr.cloud/mod.rs"]
mod auth_patr_cloud;

use std::net::SocketAddr;

use axum::{
	extract::{ConnectInfo, State},
	http::{Request, StatusCode},
	middleware::Next,
	response::Response,
	Error,
	Router,
};

use crate::{app::App, utils::plain_token_authenticator};

pub fn create_sub_route(app: &App) -> Router<App> {
	Router::new().nest("/", api_patr_cloud::create_sub_route(app))
}

async fn plain_token_authenticator_with_api_token<B>(
	State(app): State<App>,
	ip_addr: ConnectInfo<SocketAddr>,
	request: Request<B>,
	next: Next<B>,
) -> Result<Response, Error> {
	let is_api_token_allowed = true;
	// TODO - call PlainTokenAuthenticator with is_api_token_allowed
	let allowed = plain_token_authenticator(
		&app,
		&request,
		&ip_addr,
		is_api_token_allowed,
	)
	.await?;

	if allowed {
		Ok(next.run(request).await)
	} else {
		Err(Error::new(StatusCode::UNAUTHORIZED.into()))
	}
}

async fn plain_token_authenticator_without_api_token<B>(
	State(app): State<App>,
	ip_addr: ConnectInfo<SocketAddr>,
	request: Request<B>,
	next: Next<B>,
) -> Result<Response, Error> {
	let is_api_token_allowed = false;
	let allowed = plain_token_authenticator(
		&app,
		&request,
		&ip_addr,
		is_api_token_allowed,
	)
	.await?;

	if allowed {
		Ok(next.run(request).await)
	} else {
		Err(Error::new(StatusCode::UNAUTHORIZED.into()))
	}
}

pub fn get_request_ip_address<T>(
	request: &Request<T>,
	ip_addr: &ConnectInfo<SocketAddr>,
) -> String {
	let cf_connecting_ip = request
		.headers()
		.get("CF-Connecting-IP")
		.map(|value| value.to_str().unwrap().to_string()); // TODO - handle this better without unwrap()

	let x_real_ip = request
		.headers()
		.get("X-Real-IP")
		.map(|value| value.to_str().unwrap().to_string()); // TODO - handle this better without unwrap()

	let x_forwarded_for =
		request.headers().get("X-Forwarded-For").and_then(|value| {
			value
				.to_str()
				.unwrap() // TODO - handle this better without unwrap()
				.split(',')
				.next()
				.map(|ip| ip.trim().to_string())
		});

	cf_connecting_ip
		.or(x_real_ip)
		.or(x_forwarded_for)
		.unwrap_or(ip_addr.0.to_string()) // 0 represents IPv4 IP
}
