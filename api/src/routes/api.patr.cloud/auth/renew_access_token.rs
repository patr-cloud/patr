use std::ops::Add;

use argon2::{
	Algorithm,
	PasswordHash,
	PasswordHasher,
	PasswordVerifier,
	Version,
	password_hash::generate_salt,
};
use axum::http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use time::OffsetDateTime;

use crate::{models::access_token_data::AccessTokenData, prelude::*};

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
	info!(
		"Renewing access token for refresh token: `{}`",
		refresh_token.0.token()
	);

	let Some((login_id, refresh_token)) = refresh_token.0.token().split_once('.') else {
		return Err(ErrorType::MalformedRefreshToken);
	};
	trace!("Split refresh token into loginId: {login_id}");

	let login_id = Uuid::parse_str(login_id).map_err(|_| {
		debug!("loginId `{login_id}` is not a valid Uuid");
		ErrorType::MalformedRefreshToken
	})?;

	let now = OffsetDateTime::now_utc();

	// Lock the web_login row so concurrent renews are serialized — single-use
	// rotation requires verify+update to be atomic, otherwise both callers
	// pass verification before either updates the hash.
	let row = query!(
		r#"
		SELECT
			token_expiry,
			refresh_token
		FROM
			web_login
		WHERE
			login_id = $1
		FOR UPDATE;
		"#,
		login_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::MalformedRefreshToken)
	.inspect_err(|_| {
		debug!("Could not find a row for that refresh token");
	})?;

	if row.token_expiry < now {
		debug!("Token has expiry {}. It is expired.", row.token_expiry);
		return Err(ErrorType::MalformedRefreshToken);
	}

	let success = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.verify_password(
		refresh_token.as_ref(),
		&PasswordHash::new(&row.refresh_token).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying refresh token: `{}`", err);
	})
	.is_ok();

	if !success {
		debug!("Token hash could not be verified");
		return Err(ErrorType::MalformedRefreshToken);
	}

	// Rotate the refresh token. The previous one is invalidated by replacing
	// its hash, so a leaked or stale token can be used at most once.
	let new_refresh_token = Uuid::new_v4().to_string();
	let hashed_new_refresh_token = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(new_refresh_token.as_bytes(), &generate_salt())
	.inspect_err(|err| {
		error!("Error hashing refresh token: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	query!(
		r#"
		UPDATE
			web_login
		SET
			refresh_token = $1
		WHERE
			login_id = $2;
		"#,
		hashed_new_refresh_token,
		login_id as _,
	)
	.execute(&mut **database)
	.await?;

	let new_refresh_token = format!("{login_id}.{new_refresh_token}");

	let access_token = AccessTokenData {
		iss: constants::JWT_ISSUER.to_string(),
		sub: login_id,
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
