use std::ops::Add;

use argon2::{password_hash::SaltString, Algorithm, PasswordHasher, Version};
use axum::http::StatusCode;
use models::api::auth::*;
use rand::Rng;
use time::OffsetDateTime;

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
						username,
						password,
						first_name,
						last_name,
						recovery_method,
					},
			},
		database,
		redis,
		client_ip,
		config,
	}: AppRequest<'_, CreateAccountRequest>,
) -> Result<AppResponse<CreateAccountRequest>, ErrorType> {
	info!("Creating account");

	trace!("Checking if username is available");
	// check if username is available
	let is_username_available = super::is_username_valid(AppRequest {
		client_ip,
		request: ProcessedApiRequest::builder()
			.headers(IsUsernameValidRequestHeaders {
				user_agent: user_agent.clone(),
			})
			.query(IsUsernameValidQuery {
				username: username.to_string(),
			})
			.path(IsUsernameValidPath)
			.body(IsUsernameValidRequestProcessed)
			.build(),
		database,
		redis,
		config: config.clone(),
	})
	.await
	.inspect_err(|err| {
		error!("Error checking if username is available: `{}`", err);
	})?
	.body
	.available;

	if !is_username_available {
		return Err(ErrorType::UsernameUnavailable);
	}

	match &recovery_method {
		RecoveryMethod::PhoneNumber {
			recovery_phone_country_code: _,
			recovery_phone_number: _,
		} => {
			todo!("Check if phone is valid");
		}
		RecoveryMethod::Email { recovery_email } => {
			// Check if email is valid
			let is_email_available = super::is_email_valid(AppRequest {
				client_ip,
				request: ProcessedApiRequest::builder()
					.headers(IsEmailValidRequestHeaders {
						user_agent: user_agent.clone(),
					})
					.query(IsEmailValidQuery {
						email: recovery_email.clone(),
					})
					.path(IsEmailValidPath)
					.body(IsEmailValidRequestProcessed)
					.build(),
				database,
				redis,
				config: config.clone(),
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
		}
	}

	let now = OffsetDateTime::now_utc();
	let otp = format!("{:06}", rand::thread_rng().gen_range(constants::OTP_RANGE));
	let hashed_otp = argon2::Argon2::new_with_secret(
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
		otp.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.inspect_err(|err| {
		error!("Error hashing OTP: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();
	let otp_expiry = now.add(constants::OTP_VALIDITY);

	let hashed_password = argon2::Argon2::new_with_secret(
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
		password.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.inspect_err(|err| {
		error!("Error hashing password: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	let recovery_email;
	let recovery_phone_country_code;
	let recovery_phone_number;

	match recovery_method {
		RecoveryMethod::PhoneNumber {
			recovery_phone_country_code: country_code,
			recovery_phone_number: number,
		} => {
			recovery_email = None;
			recovery_phone_country_code = Some(country_code);
			recovery_phone_number = Some(number);
		}
		RecoveryMethod::Email {
			recovery_email: email,
		} => {
			recovery_email = Some(email);
			recovery_phone_country_code = None;
			recovery_phone_number = None;
		}
	}

	query!(
		r#"
		INSERT INTO
			user_to_sign_up(
				username,
				password,
				first_name,
				last_name,

				recovery_email,
				recovery_phone_country_code,
				recovery_phone_number,

				otp_hash,
				otp_expiry
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,

				$5,
				$6,
				$7,
				
				$8,
				$9
			)
		ON CONFLICT
			(username)
		DO UPDATE SET
			password = EXCLUDED.password,
			first_name = EXCLUDED.first_name,
			last_name = EXCLUDED.last_name,
			recovery_email = EXCLUDED.recovery_email,
			recovery_phone_country_code = EXCLUDED.recovery_phone_country_code,
			recovery_phone_number = EXCLUDED.recovery_phone_number,
			otp_hash = EXCLUDED.otp_hash,
			otp_expiry = EXCLUDED.otp_expiry
		WHERE
			EXCLUDED.otp_expiry > NOW();
		"#,
		&username,
		hashed_password,
		&first_name,
		&last_name,
		recovery_email,
		recovery_phone_country_code,
		recovery_phone_number,
		hashed_otp,
		otp_expiry,
	)
	.execute(&mut **database)
	.await?;

	trace!("User to sign up inserted into the database");

	// TODO send OTP via email

	AppResponse::builder()
		.body(CreateAccountResponse)
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
