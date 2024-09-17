use std::ops::Add;

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier, Version};
use http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use time::OffsetDateTime;

use crate::{prelude::*, utils::access_token_data::AccessTokenData};

pub async fn login(
	request: AppRequest<'_, LoginRequest>,
) -> Result<AppResponse<LoginRequest>, ErrorType> {
	let AppRequest {
		config,
		request:
			ProcessedApiRequest {
				path: _,
				query: _,
				headers: _,
				body: LoginRequestProcessed {
					user_id,
					password,
					mfa_otp: _,
				},
			},
		database,
	} = request;
	trace!("Logging in user: {}", user_id);

	let raw_user_data = query(
		r#"
		SELECT
			*
		FROM
			meta_data
		WHERE
			id = $1 OR
			id = $2;
		"#,
	)
	.bind(constants::USER_ID_KEY)
	.bind(constants::PASSWORD_HASH_KEY)
	.fetch_all(&mut **database)
	.await?;

	let mut user_data = UserData::new();

	for row in raw_user_data {
		let id = row.try_get::<String, &str>("id")?;
		let value = row.try_get::<String, &str>("value")?;

		match id.as_str() {
			"user_id" => {
				user_data.user_id = value;
			}
			"password_hash" => {
				user_data.password_hash = value;
			}
			_ => {}
		}
	}

	if !user_data.is_user_available() || user_data.user_id != user_id {
		return Err(ErrorType::UserNotFound);
	}

	trace!("Found user with ID: {}", user_data.user_id);

	let password_valid = Argon2::new_with_secret(
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
		password.as_bytes(),
		&PasswordHash::new(&user_data.password_hash).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying password: `{}`", err);
	})
	.is_ok();

	if !password_valid {
		return Err(ErrorType::InvalidPassword);
	}

	trace!("Password hashes match");

	let now = OffsetDateTime::now_utc();

	let access_token = AccessTokenData {
		iss: constants::JWT_ISSUER.to_string(),
		sub: user_id.to_string(),
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
	.map_err(ErrorType::server_error)?;

	// Err(ErrorType::server_error("Not implemented"))
	AppResponse::builder()
		.body(LoginResponse {
			access_token,
			refresh_token: "".to_string(),
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
