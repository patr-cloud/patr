use std::ops::Add;

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::auth::*;
use rand::RngExt;
use time::OffsetDateTime;

use crate::{prelude::*, utils::cloudflare::validate_turnstile_token};

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
						cf_turnstile_token,
					},
			},
		database,
		redis: _,
		client_ip,
		mut state,
	}: AppRequest<'_, ForgotPasswordRequest>,
) -> Result<AppResponse<ForgotPasswordRequest>, ErrorType> {
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

	if !cfg!(debug_assertions) && &cf_turnstile_response.action != "forgot-password" {
		return Err(ErrorType::TurnstileVerificationActionMismatch);
	}

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
	.await?;

	// If the user doesn't exist, return a silent 202 — same shape as the
	// success path — so the caller can't probe for account existence.
	let Some(user_data) = user_data else {
		debug!("forgot_password called for unknown user `{}`", user_id);
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
	.inspect_err(|err| {
		error!("Error creating Argon2: `{err}`");
	})
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(password_reset_token.as_bytes(), &generate_salt())
	.inspect_err(|err| {
		error!("Error hashing reset token: `{err}`");
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

	// Deliberately do NOT reset password_reset_attempts here. Zeroing on
	// every re-request would let a slow-drip attacker pull a fresh
	// MAX_PASSWORD_RESET_ATTEMPTS budget every OTP cycle. The counter is
	// already nulled in reset_password on a successful reset.
	query!(
		r#"
		UPDATE
			"user"
		SET
			password_reset_token = $1,
			password_reset_token_expiry = $2
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

	// Deliver the OTP via the user's chosen recovery channel. Email is wired
	// today; SMS is left as a TODO until the SMS sender is available.
	match preferred_recovery_option {
		PreferredRecoveryOption::RecoveryEmail => {
			if let Some(recovery_email) = user_data.recovery_email {
				state
					.worker
					.send_email(
						recovery_email.clone(),
						ForgotPasswordEmail {
							username: user_data.username,
							email: recovery_email,
							otp_code: password_reset_token,
							otp_expiry: constants::OTP_VALIDITY.to_string(),
						},
					)
					.await?;
			}
		}
		PreferredRecoveryOption::RecoveryPhoneNumber => {
			// TODO: dispatch the OTP to the user's recovery phone once the
			// SMS sender is wired up. Until then, a phone-only user will
			// hit this branch silently — same shape as the existing
			// silent-success paths above, so we don't leak that detail.
		}
	}

	AppResponse::builder()
		.body(ForgotPasswordResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
