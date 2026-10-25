use std::ops::Add;

use axum::http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use sha2::{Digest as _, Sha256};
use time::OffsetDateTime;

use crate::{
	models::{access_token_data::AccessTokenData, permissions},
	prelude::*,
};

pub async fn renew_access_token(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: RenewAccessTokenPath,
				query: (),
				headers:
					RenewAccessTokenRequestHeaders {
						refresh_token,
						user_agent: _,
					},
				body: RenewAccessTokenRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		state,
	}: AppRequest<'_, RenewAccessTokenRequest>,
) -> Result<AppResponse<RenewAccessTokenRequest>, ErrorType> {
	info!("Renewing access token");

	let refresh_token = refresh_token.0.token();
	if !refresh_token.starts_with(constants::REFRESH_TOKEN_PREFIX) ||
		!permissions::has_valid_checksum(refresh_token)
	{
		debug!("Not a refresh token");
		return Err(ErrorType::MalformedRefreshToken);
	}

	let now = OffsetDateTime::now_utc();

	// Lock the web_login row so concurrent renews are serialized — single-use
	// rotation requires lookup+update to be atomic, otherwise both callers
	// find the row before either replaces the hash.
	let row = query!(
		r#"
		SELECT
			login_id,
			token_expiry,
			user_id
		FROM
			web_login
		WHERE
			token_hash = $1
		FOR UPDATE;
		"#,
		hex::encode(Sha256::digest(refresh_token)),
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::MalformedRefreshToken)
	.inspect_err(|_| {
		debug!("Could not find a row for that refresh token");
	})?;
	let login_id = row.login_id;

	if row.token_expiry < now {
		debug!("Token has expiry {}. It is expired.", row.token_expiry);
		return Err(ErrorType::MalformedRefreshToken);
	}

	// Rotate the refresh token. The previous one is invalidated by replacing
	// its hash, so a leaked or stale token can be used at most once.
	let new_refresh_token = permissions::generate_refresh_token();

	query!(
		r#"
		UPDATE
			web_login
		SET
			token_hash = $1
		WHERE
			login_id = $2;
		"#,
		hex::encode(Sha256::digest(&new_refresh_token)),
		login_id,
	)
	.execute(&mut **database)
	.await?;

	let access_token = AccessTokenData {
		iss: constants::JWT_ISSUER.to_string(),
		sub: row.user_id,
		sid: login_id,
		aud: OneOrMore::One(constants::PATR_JWT_AUDIENCE.to_string()),
		exp: now.add(constants::ACCESS_TOKEN_VALIDITY),
		nbf: now,
		iat: now,
		jti: Uuid::now_v1(),
	};

	let access_token = jsonwebtoken::encode(
		&Default::default(),
		&access_token,
		&EncodingKey::from_secret(state.config.jwt_secret.as_ref()),
	)
	.inspect_err(|err| {
		error!("Error encoding JWT: `{}`", err);
	})?;

	trace!("Access token generated");

	AppResponse::builder()
		.body(RenewAccessTokenResponse {
			access_token,
			refresh_token: new_refresh_token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
