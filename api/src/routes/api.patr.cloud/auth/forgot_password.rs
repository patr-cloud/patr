use std::ops::Add;

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::auth::*;
use rand::RngExt;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn forgot_password(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ForgotPasswordPath,
				query: (),
				headers: ForgotPasswordRequestHeaders { user_agent: _ },
				body: ForgotPasswordRequestProcessed { email },
			},
		database,
		redis: _,
		client_ip: _,
		state,
	}: AppRequest<'_, ForgotPasswordRequest>,
) -> Result<AppResponse<ForgotPasswordRequest>, ErrorType> {
	info!("Initiating forgot password request for: `{email}`");

	let user_data = query!(
		r#"
		SELECT
			"user".id,
			"user".password_reset_token_expiry
		FROM
			"user"
		WHERE
			"user".email = $1;
		"#,
		&email,
	)
	.fetch_optional(&mut **database)
	.await?;

	// If the user doesn't exist, return a silent 202 — same shape as the
	// success path — so the caller can't probe for account existence.
	let Some(user_data) = user_data else {
		debug!("forgot_password called for unknown email `{}`", email);
		return AppResponse::builder()
			.body(ForgotPasswordResponse)
			.headers(())
			.status_code(StatusCode::ACCEPTED)
			.build()
			.into_result();
	};

	let now = OffsetDateTime::now_utc();
	let password_reset_token = format!("{:06}", rand::rng().random_range(constants::OTP_RANGE));
	let password_reset_token_expiry = now.add(constants::OTP_VALIDITY);
	let hashed_password_reset_token = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(password_reset_token.as_bytes(), &generate_salt())
	.map_err(ErrorType::server_error)?
	.to_string();

	if user_data
		.password_reset_token_expiry
		.unwrap_or(OffsetDateTime::UNIX_EPOCH) >
		now
	{
		debug!("User has an active password reset token");

		return AppResponse::builder()
			.body(ForgotPasswordResponse)
			.headers(())
			.status_code(StatusCode::ACCEPTED)
			.build()
			.into_result();
	}

	query!(
		r#"
		UPDATE
			"user"
		SET
			password_reset_token = $1,
			password_reset_token_expiry = $2,
			password_reset_attempts = 0
		WHERE
			id = $3;
		"#,
		hashed_password_reset_token,
		password_reset_token_expiry,
		user_data.id,
	)
	.execute(&mut **database)
	.await?;

	trace!("Password reset token for user `{}` updated", user_data.id);

	// TODO send OTP via email

	AppResponse::builder()
		.body(ForgotPasswordResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
