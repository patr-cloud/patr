use std::ops::Add;

use argon2::{password_hash::SaltString, Algorithm, PasswordHasher, Version};
use axum::http::StatusCode;
use models::api::auth::*;
use rand::Rng;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn forgot_password(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ForgotPasswordPath,
				query: (),
				headers: ForgotPasswordRequestHeaders { user_agent: _ },
				body:
					ForgotPasswordRequestProcessed {
						user_id,
						preferred_recovery_option,
					},
			},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ForgotPasswordRequest>,
) -> Result<AppResponse<ForgotPasswordRequest>, ErrorType> {
	info!("Initiating forgot password request for user: `{user_id}`");

	let user_data = query!(
		r#"
		SELECT
			"user".id,
			"user".username,
			"user".password,
            "user".recovery_email,
            "user".recovery_phone_country_code,
            "user".recovery_phone_number,
            "user".password_reset_token_expiry
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
	let password_reset_token = rand::thread_rng()
		.gen_range(constants::OTP_RANGE)
		.to_string();
	let password_reset_token_expiry = now.add(constants::OTP_VALIDITY);
	let hashed_password_reset_token = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{err}");
	})
	.map_err(ErrorType::server_error)?
	.hash_password(
		password_reset_token.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.inspect_err(|err| {
		error!("Error hashing reset token: `{err}");
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	let should_reset = match &preferred_recovery_option {
		PreferredRecoveryOption::RecoveryPhoneNumber => user_data.recovery_phone_number.is_some(),
		PreferredRecoveryOption::RecoveryEmail => user_data.recovery_email.is_some(),
	};

	if !should_reset {
		debug!("User has selected a recovery option that is not set in the database");

		// Return Ok even if the data is invalid to prevent leaking user data
		return AppResponse::builder()
			.body(ForgotPasswordResponse)
			.headers(())
			.status_code(StatusCode::ACCEPTED)
			.build()
			.into_result();
	}

	if user_data
		.password_reset_token_expiry
		.unwrap_or(OffsetDateTime::UNIX_EPOCH) >
		now
	{
		debug!("User has an active password reset token");

		// The previous attempt hasn't expired yet
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

	// TODO send OTP by the preferred recovery option

	AppResponse::builder()
		.body(ForgotPasswordResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
