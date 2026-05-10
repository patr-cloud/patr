use argon2::{
	Algorithm,
	PasswordHash,
	PasswordHasher,
	PasswordVerifier,
	Version,
	password_hash::generate_salt,
};
use axum::http::StatusCode;
use models::api::auth::*;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn reset_password(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ResetPasswordPath,
				query: (),
				headers: ResetPasswordRequestHeaders { user_agent: _ },
				body:
					ResetPasswordRequestProcessed {
						email,
						verification_token,
						password,
					},
			},
		database,
		redis: _,
		client_ip: _,
		state,
	}: AppRequest<'_, ResetPasswordRequest>,
) -> Result<AppResponse<ResetPasswordRequest>, ErrorType> {
	info!("Resetting password for user: `{email}`");

	let user_data = query!(
		r#"
		SELECT
			"user".id,
			"user".password_reset_token,
			"user".password_reset_token_expiry,
			"user".password_reset_attempts
		FROM
			"user"
		WHERE
			"user".email = $1;
		"#,
		&email,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	let now = OffsetDateTime::now_utc();

	if user_data
		.password_reset_token_expiry
		.unwrap_or(OffsetDateTime::UNIX_EPOCH) <
		now
	{
		debug!("Password reset token has expired");
		return Err(ErrorType::InvalidPasswordResetToken);
	}

	if user_data.password_reset_attempts.unwrap_or(0) > constants::MAX_PASSWORD_RESET_ATTEMPTS {
		debug!("Password reset attempts exceeded");
		return Err(ErrorType::InvalidPasswordResetToken);
	}

	query!(
		r#"
		UPDATE
			"user"
		SET
			password_reset_attempts = password_reset_attempts + 1
		WHERE
			id = $1;
		"#,
		user_data.id
	)
	.execute(&mut **database)
	.await?;

	let Some(password_reset_token) = user_data.password_reset_token else {
		debug!("Password reset token is missing");
		return Err(ErrorType::InvalidPasswordResetToken);
	};

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
		verification_token.as_bytes(),
		&PasswordHash::new(&password_reset_token).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying token: `{}`", err);
	})
	.is_ok();

	if !success {
		return Err(ErrorType::InvalidPasswordResetToken);
	}

	let hashed_password = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(password.as_bytes(), &generate_salt())
	.inspect_err(|err| {
		error!("Error hashing password: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	query!(
		r#"
		UPDATE
			"user"
		SET
			password = $1
		WHERE
			id = $2;
		"#,
		hashed_password,
		user_data.id
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(ResetPasswordResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
