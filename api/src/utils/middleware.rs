use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// use anyhow::Error;
use axum::{
	extract::ConnectInfo,
	http::{HeaderMap, Request, StatusCode},
	Error,
};

use crate::{
	app::App,
	db::Resource,
	models::UserAuthenticationData,
	routes::get_request_ip_address,
};

pub async fn plain_token_authenticator<T>(
	app: &App,
	request: &Request<T>,
	addr: &ConnectInfo<SocketAddr>,
	is_api_token_allowed: bool,
) -> Result<bool, Error> {
	let token = if let Some(token) = request.headers().get("Authorization") {
		token.to_str().unwrap() // Handle unwrap case adn return Unautorized case
	} else {
		return Err(Error::new(StatusCode::UNAUTHORIZED.into()));
	};

	let ip_addr = get_request_ip_address(request, addr)
		.parse()
		.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

	let jwt_secret = app.config.jwt_secret.clone();

	//  TODO DB connection and redis connection
	let mut redis_conn = app.redis;
	let token_data = match UserAuthenticationData::parse(
		context.get_database_connection(),
		&mut redis_conn,
		&jwt_secret,
		&token,
		&ip_addr,
	)
	.await
	{
		Ok(token_data) => token_data,
		Err(err) => {
			log::error!(
				"Error while parsing user authentication data: {}",
				err
			);
			return Err(Error::new(StatusCode::INTERNAL_SERVER_ERROR.into()));
		}
	};

	if token_data.is_api_token() && !is_api_token_allowed {
		return Err(Error::new(StatusCode::UNAUTHORIZED.into()));
	}

	// TODO - set token data for the app
	// context.set_token_data(token_data);
	Ok(true)
}

pub async fn resource_token_authenticator<T>(
	app: &App,
	request: &Request<T>,
	addr: &ConnectInfo<SocketAddr>,
	is_api_token_allowed: bool,
	resource: &Resource,
	permission: &str,
) -> Result<bool, Error> {
	let token = if let Some(token) = request.headers().get("Authorization") {
		token.to_str().unwrap()
	} else {
		return Err(Error::new(StatusCode::UNAUTHORIZED.into()));
	};

	let ip_addr = get_request_ip_address(request, addr)
		.parse()
		.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

	let jwt_secret = app.config.jwt_secret.clone();

	//  TODO DB connection and redis connection
	let mut redis_conn = app.redis;
	let token_data = match UserAuthenticationData::parse(
		context.get_database_connection(),
		&mut redis_conn,
		&jwt_secret,
		&token,
		&ip_addr,
	)
	.await
	{
		Ok(token_data) => token_data,
		Err(err) => {
			log::error!(
				"Error while parsing user authentication data: {}",
				err
			);
			return Err(Error::new(StatusCode::INTERNAL_SERVER_ERROR.into()));
		}
	};

	if token_data.is_api_token() && !is_api_token_allowed {
		return Err(Error::new(StatusCode::UNAUTHORIZED.into()));
	}

	if !token_data.has_access_for_requested_action(
		&resource.owner_id,
		&resource.id,
		&resource.resource_type_id,
		permission,
	) {
		return Err(Error::new(StatusCode::UNAUTHORIZED.into()));
	}

	// TODO - set token data for the app
	// context.set_token_data(token_data);
	Ok(true)
}
