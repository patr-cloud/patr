use axum::{http::StatusCode, Router};
use models::{api::auth::*, ErrorType};

use crate::prelude::*;

mod login;
mod logout;

use self::{login::*, logout::*};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(login, state)
		.mount_auth_endpoint(logout, state)
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
