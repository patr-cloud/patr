use axum::Router;

use crate::prelude::*;

mod complete_sign_up;
mod create_account;
mod forgot_password;
mod is_email_valid;
mod is_username_valid;
mod list_recovery_options;
mod login;
mod logout;
#[expect(unused_variables)]
mod oauth;
mod renew_access_token;
mod resend_otp;
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
	resend_otp::*,
	reset_password::*,
};

/// Sets up the auth routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.merge(oauth::setup_routes(state, allowed_client_type).await)
		.mount_endpoint(login, state, allowed_client_type)
		.mount_auth_endpoint(logout, state, allowed_client_type)
		.mount_endpoint(create_account, state, allowed_client_type)
		.mount_endpoint(renew_access_token, state, allowed_client_type)
		.mount_endpoint(forgot_password, state, allowed_client_type)
		.mount_endpoint(is_email_valid, state, allowed_client_type)
		.mount_endpoint(is_username_valid, state, allowed_client_type)
		.mount_endpoint(complete_sign_up, state, allowed_client_type)
		.mount_endpoint(list_recovery_options, state, allowed_client_type)
		.mount_endpoint(resend_otp, state, allowed_client_type)
		.mount_endpoint(reset_password, state, allowed_client_type)
}
