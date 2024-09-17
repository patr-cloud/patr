use std::ops::Add;

use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
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
		config,
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

	let row = query!(
		r#"
        SELECT
            token_expiry,
			refresh_token
        FROM
            web_login
        WHERE
            login_id = $1;
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
		config.password_pepper.as_ref(),
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
		&EncodingKey::from_secret(config.jwt_secret.as_ref()),
	)
	.inspect_err(|err| {
		error!("Error encoding JWT: `{}`", err);
	})?;

	trace!("Access token generated");

	AppResponse::builder()
		.body(RenewAccessTokenResponse { access_token })
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
