use axum::{http::StatusCode, Router};
use models::api::auth::*;

use crate::prelude::*;

mod complete_sign_up;
mod create_account;
mod forgot_password;
mod is_email_valid;
mod is_username_valid;
mod list_recovery_options;
mod login;
mod logout;
mod renew_access_token;
mod reset_password;

use self::{
	complete_sign_up::*,
	create_account::*,
	forgot_password::*,
	is_email_valid::*,
	is_username_valid::*,
	list_recovery_options::*,
	login::*,
	logout::*,
	renew_access_token::*,
	reset_password::*,
};

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
