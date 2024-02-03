use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::{http::StatusCode, Router};
use models::{api::auth::*, ErrorType};
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(login, state)
		.mount_endpoint(logout, state)
		.mount_endpoint(create_account, state)
		.mount_endpoint(renew_access_token, state)
		.mount_endpoint(forgot_password, state)
		.mount_endpoint(is_email_valid, state)
		.mount_endpoint(is_username_valid, state)
		.mount_endpoint(complete_sign_up, state)
		.mount_endpoint(list_recovery_options, state)
		.mount_endpoint(resend_otp, state)
		.mount_endpoint(reset_password, state)
		.with_state(state.clone())
}

async fn login(
	AppRequest {
		request: ProcessedApiRequest {
			path: _,
			query: _,
			headers: _,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, LoginRequest>,
) -> Result<AppResponse<LoginRequest>, ErrorType> {
	let user_data = query!(
		r#"
		SELECT
			"user".id,
			"user".username,
			"user".password,
			"user".mfa_secret
		FROM
			"user"
		LEFT JOIN
			personal_email
		ON
			personal_email.user_id = "user".id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id
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
			CONCAT(
				personal_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1 OR
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = $1;
		"#,
		""
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	let success = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(|err| ErrorType::server_error(err.to_string()))?
	.verify_password(
		body.password.as_ref(),
		&PasswordHash::new(&user_data.password)
			.map_err(|err| ErrorType::server_error(err.to_string()))?,
	)
	.is_ok();

	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	if let Some(mfa_secret) = user_data.mfa_secret {
		let Some(mfa_otp) = body.mfa_otp else {
			return Err(ErrorType::MfaRequired);
		};

		let totp = TOTP::new(
			TotpAlgorithm::SHA1,
			6,
			1,
			30,
			Secret::Encoded(mfa_secret).to_bytes().map_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
				ErrorType::server_error(err.to_string())
			})?,
		)
		.map_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
			ErrorType::server_error(err.to_string())
		})?;

		let mfa_valid = totp.check_current(&mfa_otp).map_err(|err| {
			error!(
				"System time error while checking TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
			ErrorType::server_error(err.to_string())
		})?;

		if !mfa_valid {
			return Err(ErrorType::MfaOtpInvalid);
		}
	}

	// TODO: Generate Login in DB
	// TODO: Generate Token and send it as response

	AppResponse::builder()
		.body(LoginResponse {
			access_token: "TODO".to_string(),
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}

async fn logout(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, LogoutRequest>,
) -> Result<AppResponse<LogoutRequest>, ErrorType> {
	info!("Starting: Create account");

	// LOGIC

	AppResponse::builder()
		.body(LogoutResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn create_account(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, CreateAccountRequest>,
) -> Result<AppResponse<CreateAccountRequest>, ErrorType> {
	info!("Starting: Create account");

	// LOGIC

	AppResponse::builder()
		.body(CreateAccountResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn renew_access_token(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, RenewAccessTokenRequest>,
) -> Result<AppResponse<RenewAccessTokenRequest>, ErrorType> {
	info!("Starting: Create account");

	// LOGIC

	AppResponse::builder()
		.body(RenewAccessTokenResponse {
			access_token: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn forgot_password(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ForgotPasswordRequest>,
) -> Result<AppResponse<ForgotPasswordRequest>, ErrorType> {
	info!("Starting: Forget password");

	// LOGIC

	AppResponse::builder()
		.body(ForgotPasswordResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn is_email_valid(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, IsEmailValidRequest>,
) -> Result<AppResponse<IsEmailValidRequest>, ErrorType> {
	info!("Starting: Check for email validity");

	// LOGIC

	AppResponse::builder()
		.body(IsEmailValidResponse { available: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn is_username_valid(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, IsUsernameValidRequest>,
) -> Result<AppResponse<IsUsernameValidRequest>, ErrorType> {
	info!("Starting: Check for username validity");

	// LOGIC

	AppResponse::builder()
		.body(IsUsernameValidResponse { available: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn complete_sign_up(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, CompleteSignUpRequest>,
) -> Result<AppResponse<CompleteSignUpRequest>, ErrorType> {
	info!("Starting: Complete sign up");

	// LOGIC

	AppResponse::builder()
		.body(CompleteSignUpResponse {
			access_token: todo!(),
			refresh_token: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_recovery_options(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ListRecoveryOptionsRequest>,
) -> Result<AppResponse<ListRecoveryOptionsRequest>, ErrorType> {
	info!("Starting: List recovery options");

	// LOGIC

	AppResponse::builder()
		.body(ListRecoveryOptionsResponse {
			recovery_phone_number: todo!(),
			recovery_email: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn resend_otp(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ResendOtpRequest>,
) -> Result<AppResponse<ResendOtpRequest>, ErrorType> {
	info!("Starting: Resend OTP");

	// LOGIC

	AppResponse::builder()
		.body(ResendOtpResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn reset_password(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ResetPasswordRequest>,
) -> Result<AppResponse<ResetPasswordRequest>, ErrorType> {
	info!("Starting: Reset password");

	// LOGIC

	AppResponse::builder()
		.body(ResetPasswordResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
