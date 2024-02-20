use std::ops::Add;

use argon2::{password_hash::SaltString, Algorithm, PasswordHasher, Version};
use axum::http::StatusCode;
use models::{api::auth::*, ApiRequest, ErrorType};
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
				username: username.clone(),
			})
			.path(IsUsernameValidPath)
			.body(IsUsernameValidRequestProcessed)
			.build(),
		database,
		redis,
		config: config.clone(),
	})
	.await?
	.body
	.available;

	if !is_username_available {
		return Err(ErrorType::UsernameUnavailable);
	}

	match &recovery_method {
		RecoveryMethod::PhoneNumber {
			recovery_phone_country_code,
			recovery_phone_number,
		} => {
			// TODO Check if phone is valid
			true;
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
			.await?
			.body
			.available;

			if !is_email_available {
				return Err(ErrorType::EmailUnavailable);
			}
		}
	}

	let now = OffsetDateTime::now_utc();
	let otp = rand::thread_rng().gen_range(constants::OTP_RANGE);
	let hashed_otp = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password(
		otp.to_string().as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.map_err(ErrorType::server_error)?
	.to_string();
	let otp_expiry = now.add(constants::OTP_VALIDITY);

	let hashed_password = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password(
		password.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.map_err(ErrorType::server_error)?
	.to_string();

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
			);
		"#,
		username,
		hashed_password,
		first_name,
		last_name,
		"TODO",
		"TODO",
		"TODO",
		hashed_otp,
		otp_expiry,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(CreateAccountResponse)
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
