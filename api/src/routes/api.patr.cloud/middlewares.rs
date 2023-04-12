use api_models::Error;
use axum::{
	extract::{FromRequestParts, State},
	http::{HeaderMap, Request},
	middleware::Next,
	response::Response,
	TypedHeader,
};

use crate::{
	app::{App, DbTx},
	models::UserAuthenticationData,
	routes::ClientIp,
};

pub async fn plain_token_authenticator<B>(
	State(mut app): State<App>,
	mut db_tx: DbTx,
	ClientIp(client_ip): ClientIp,
	mut req: Request<B>,
	next: Next<B>,
) -> Result<Response, Error> {
	let auth_header_token = req
		.headers()
		.get(http::header::AUTHORIZATION)
		.and_then(|hv| hv.to_str().ok())
		.ok_or(Error::Unauthorized)?;

	let token_data = UserAuthenticationData::parse(
		&mut db_tx,
		&mut app.redis,
		&app.config.jwt_secret,
		&auth_header_token,
		&client_ip,
	)
	.await
	.map_err(|err| {
		log::info!("Error while parsing user authentication data: {}", err);
		Error::Unauthorized
	})?;

	// now add token_data to extenstions so that it can be used later by handler
	req.extensions_mut().insert(token_data);

	// drop db_tx so that tx can be borrowed later from extensions by handler
	drop(db_tx);

	Ok(next.run(req).await)
}

// TODO:
// plain token authenticator won't validate whether the token hoas perission
// for restricting api token add a seperate middleware
