use argon2::{password_hash::SaltString, Algorithm, PasswordHasher, Version};
use models::api::user::*;
use reqwest::StatusCode;

use crate::prelude::*;

pub async fn regenerate_api_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RegenerateApiTokenPath { token_id },
				query: (),
				headers:
					RegenerateApiTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RegenerateApiTokenRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedAppRequest<'_, RegenerateApiTokenRequest>,
) -> Result<AppResponse<RegenerateApiTokenRequest>, ErrorType> {
	trace!("Regenerating API token: {}", token_id);

	let refresh_token = Uuid::new_v4();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.hash_password(
		refresh_token.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.inspect_err(|err| {
		error!("Error hashing refresh token: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	query!(
		r#"
		UPDATE
			user_api_token
		SET
			token_hash = $1
		WHERE
			token_id = $2 AND
			user_id = $3;
		"#,
		hashed_refresh_token,
		token_id as _,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(RegenerateApiTokenResponse {
			token: format!("patrv1.{}.{}", refresh_token, token_id),
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
