use models::api::auth::*;

use crate::prelude::*;

pub async fn create_account(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: CreateAccountPath,
				query: (),
				headers: CreateAccountRequestHeaders { user_agent },
				body:
					CreateAccountRequestProcessed {
						email,
						password,
						first_name,
						last_name,
						cf_turnstile_token,
					},
			},
		database,
		redis,
		client_ip,
		mut state,
	}: AppRequest<'_, CreateAccountRequest>,
) -> Result<AppResponse<CreateAccountRequest>, ErrorType> {
	cfg_if! {
		if #[cfg(not(feature = "cloud"))] {
			// Self-hosted instances don't allow self-service sign-up — users
			// are seeded / invited by the operator. Mirror the frontend, which
			// 404s the sign-up routes.
			let _ = (
				email,
				password,
				first_name,
				last_name,
				cf_turnstile_token,
				user_agent,
				database,
				redis,
				client_ip,
				state,
			);
			Err(ErrorType::FeatureNotSupported)
		} else {
			use std::ops::Add;

			use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
			use axum::http::StatusCode;
			use rand::RngExt;
			use time::OffsetDateTime;

			use crate::utils::cloudflare::validate_turnstile_token;

			trace!("Validating Cloudflare Turnstile token");
			let cf_turnstile_response = validate_turnstile_token(
				&state.config.cloudflare.turnstile_secret,
				&cf_turnstile_token,
				client_ip,
			)
			.await
			.inspect_err(|err| {
				error!("Error verifying Cloudflare Turnstile token: `{}`", err);
			})?;

			if !cf_turnstile_response.success {
				return Err(ErrorType::TurnstileVerificationFailed);
			}

			if !cfg!(debug_assertions) && &cf_turnstile_response.action != "sign-up" {
				return Err(ErrorType::TurnstileVerificationActionMismatch);
			}

			info!("Creating account");

			trace!("Checking if the email is available");
			let is_email_available = super::is_email_valid(AppRequest {
				client_ip,
				request: ProcessedApiRequest::builder()
					.headers(IsEmailValidRequestHeaders {
						user_agent: user_agent.clone(),
					})
					.query(IsEmailValidQueryProcessed {
						email: email.clone(),
					})
					.path(IsEmailValidPath)
					.body(IsEmailValidRequestProcessed)
					.build(),
				database,
				redis,
				state: state.clone(),
			})
			.await
			.inspect_err(|err| {
				error!("Error checking if email is available: `{}`", err);
			})?
			.body
			.available;

			if !is_email_available {
				return Err(ErrorType::EmailUnavailable);
			}

			let now = OffsetDateTime::now_utc();
			let otp = format!("{:06}", rand::rng().random_range(constants::OTP_RANGE));
			let hashed_otp = argon2::Argon2::new_with_secret(
				state.config.password_pepper.as_ref(),
				Algorithm::Argon2id,
				Version::V0x13,
				constants::HASHING_PARAMS,
			)
			.inspect_err(|err| {
				error!("Error creating Argon2: `{}`", err);
			})
			.map_err(ErrorType::server_error)?
			.hash_password_with_salt(otp.as_bytes(), &generate_salt())
			.inspect_err(|err| {
				error!("Error hashing OTP: `{}`", err);
			})
			.map_err(ErrorType::server_error)?
			.to_string();
			let otp_expiry = now.add(constants::OTP_VALIDITY);

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
				INSERT INTO
					user_to_sign_up(
						email,
						password,
						first_name,
						last_name,

						otp_hash,
						otp_expiry,
						sign_up_attempts
					)
				VALUES
					(
						$1,
						$2,
						$3,
						$4,

						$5,
						$6,
						0
					)
				ON CONFLICT
					(email)
				DO UPDATE SET
					password = EXCLUDED.password,
					first_name = EXCLUDED.first_name,
					last_name = EXCLUDED.last_name,
					otp_hash = EXCLUDED.otp_hash,
					otp_expiry = EXCLUDED.otp_expiry
				WHERE
					EXCLUDED.otp_expiry > NOW();
				"#,
				&email,
				hashed_password,
				&first_name,
				&last_name,
				hashed_otp,
				otp_expiry,
			)
			.execute(&mut **database)
			.await
			.inspect_err(|err| {
				error!("Error inserting into user_to_sign_up: `{}`", err);
			})?;

			trace!("User to sign up inserted into the database");

			state
				.worker
				.send_email(
					email.to_string(),
					UserSignUpEmail {
						first_name: first_name.to_string(),
						email: email.to_string(),
						otp,
						otp_expiry: constants::OTP_VALIDITY.to_string(),
					},
				)
				.await
				.inspect_err(|err| {
					error!("Error enqueuing sign-up email: `{}`", err);
				})?;

			AppResponse::builder()
				.body(CreateAccountResponse)
				.headers(())
				.status_code(StatusCode::CREATED)
				.build()
				.into_result()
		}
	}
}
