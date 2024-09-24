use std::ops::Add;

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier, Version};
use http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use time::OffsetDateTime;

use crate::{prelude::*, utils::access_token_data::AccessTokenData};

/// The handler to login the user. This will return the access token and the
/// refresh token.
pub async fn login(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: LoginPath,
				query: (),
				headers: LoginRequestHeaders { user_agent: _ },
				body: LoginRequestProcessed {
					user_id,
					password,
					mfa_otp: _,
				},
			},
		database,
		runner_changes_sender: _,
		config,
	}: AppRequest<'_, LoginRequest>,
) -> Result<AppResponse<LoginRequest>, ErrorType> {
	trace!("Logging in user: {}", user_id);

	let rows = query(
		r#"
		SELECT
			*
		FROM
			meta_data
		WHERE
			(
				id = $1 AND
				value = $2
	 		) OR
			id = $3;
		"#,
	)
	.bind(constants::USER_ID_KEY)
	.bind(user_id.as_ref())
	.bind(constants::PASSWORD_HASH_KEY)
	.fetch_all(&mut **database)
	.await?;

	let mut db_user_id = None;
	let mut db_password_hash = None;

	for row in rows {
		let id = row.try_get::<String, _>("id")?;
		let value = row.try_get::<String, _>("value")?;

		match id.as_str() {
			constants::USER_ID_KEY => {
				db_user_id = Some(value);
			}
			constants::PASSWORD_HASH_KEY => {
				db_password_hash = Some(value);
			}
			_ => (),
		}
	}

	let Some((user_id, password_hash)) = db_user_id.zip(db_password_hash) else {
		return Err(ErrorType::UserNotFound);
	};

	trace!("Found user with ID: {}", user_id);

	let RunnerMode::SelfHosted {
		password_pepper,
		jwt_secret,
	} = config.mode
	else {
		return Err(ErrorType::InvalidRunnerMode);
	};

	let password_valid = Argon2::new_with_secret(
		password_pepper.as_ref(),
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
		&PasswordHash::new(&password_hash).map_err(ErrorType::server_error)?,
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
		&EncodingKey::from_secret(jwt_secret.as_ref()),
	)
	.map_err(ErrorType::server_error)?;

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
