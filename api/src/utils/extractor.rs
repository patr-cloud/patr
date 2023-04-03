use std::net::IpAddr;

use axum::{
	async_trait,
	extract::FromRequestParts,
	http::{header::AUTHORIZATION, request::Parts, StatusCode},
	RequestPartsExt,
};
use axum_sqlx_tx::Tx;
use sqlx::Postgres;

use crate::{
	app::App,
	models::UserAuthenticationData,
	routes::get_request_ip_address,
};

#[async_trait]
impl FromRequestParts<App> for UserAuthenticationData {
	type Rejection = (StatusCode, &'static str);

	async fn from_request_parts(
		parts: &mut Parts,
		state: &App,
		// state: &S,
	) -> Result<Self, Self::Rejection> {
		type Rejection = (StatusCode, &'static str);

		let ip_addr = get_request_ip_address(&parts);
		let mut connection = parts.extract::<Tx<Postgres>>().await?;

		if let Some(token) = parts.headers.get(AUTHORIZATION) {
			let token = token.to_str().ok().unwrap(); // TODO - remove this unwrap(), this is for testing
			let token_data = UserAuthenticationData::parse(
				&mut connection,
				&mut state.redis,
				&state.config.jwt_secret,
				token,
				&ip_addr.parse::<IpAddr>()?,
			)
			.await?;
			Ok(token_data)
		} else {
			Err((StatusCode::BAD_REQUEST, "`Authorization` header is missing"))
		}
	}
}
