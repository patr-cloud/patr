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
use rustis::commands::StringCommands as _;
use time::OffsetDateTime;

use crate::{prelude::*, redis::keys as redis, utils::cloudflare::validate_turnstile_token};

pub async fn reset_password(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ResetPasswordPath,
				query: (),
				headers: ResetPasswordRequestHeaders { user_agent: _ },
				body:
					ResetPasswordRequestProcessed {
						user_id,
						verification_token,
						password,
						cf_turnstile_token,
					},
			},
		database,
		redis,
		client_ip,
		state,
	}: AppRequest<'_, ResetPasswordRequest>,
) -> Result<AppResponse<ResetPasswordRequest>, ErrorType> {
	trace!("Validating Cloudflare Turnstile token");
	let cf_turnstile_response = validate_turnstile_token(
		&state.config.cloudflare.turnstile_secret,
		&cf_turnstile_token,
		Some(client_ip),
	)
	.await
	.inspect_err(|err| {
		error!("Error verifying Cloudflare Turnstile token: `{}`", err);
	})?;

	if !cf_turnstile_response.success {
		return Err(ErrorType::TurnstileVerificationFailed);
	}

	if !cfg!(debug_assertions) && &cf_turnstile_response.action != "reset-password" {
		return Err(ErrorType::TurnstileVerificationActionMismatch);
	}

	info!("Resetting password for user: `{user_id}`");

	let user_data = query!(
		r#"
		SELECT
			"user".id,
			"user".password_reset_token,
			"user".password_reset_token_expiry,
			"user".password_reset_attempts
		FROM
			"user"
		LEFT JOIN
			user_email
		ON
			user_email.user_id = "user".id
		LEFT JOIN
			user_phone_number
		ON
			user_phone_number.user_id = "user".id
		LEFT JOIN
			phone_number_country_code
		ON
			phone_number_country_code.country_code = user_phone_number.country_code
		WHERE
			"user".username = $1 OR
			user_email.email = $1 OR
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = $1;
		"#,
		&user_id,
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

	if user_data.password_reset_attempts.unwrap_or(0) >= constants::MAX_PASSWORD_RESET_ATTEMPTS {
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

	// Update the password and consume the reset token in one statement so the
	// same OTP can't be replayed within its TTL.
	query!(
		r#"
		UPDATE
			"user"
		SET
			password = $1,
			password_reset_token = NULL,
			password_reset_token_expiry = NULL,
			password_reset_attempts = NULL
		WHERE
			id = $2;
		"#,
		hashed_password,
		user_data.id
	)
	.execute(&mut **database)
	.await?;

	// Reset has no caller session, so drop every web login the user had.
	query!(
		r#"
		DELETE FROM
			web_login
		WHERE
			user_id = $1;
		"#,
		user_data.id,
	)
	.execute(&mut **database)
	.await?;

	redis
		.setex(
			redis::user_id_revocation_timestamp(&user_data.id.into()),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting user_id_revocation_timestamp: `{}`", err);
		})?;

	AppResponse::builder()
		.body(ResetPasswordResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
