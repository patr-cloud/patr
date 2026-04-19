use axum::Router;

use crate::prelude::*;

/// The route to complete the sign up process
mod complete_sign_up;
/// The route to create a new user account
mod create_account;
/// The route to login using Docker credentials
mod docker_login;
/// The route to initiate the forgot password process
mod forgot_password;
/// The route to check if an email is valid
mod is_email_valid;
/// The route to check if a username is valid
mod is_username_valid;
/// The route to list recovery options for a user
mod list_recovery_options;
/// The route to login a user
mod login;
/// The route to logout a user
mod logout;
/// All OAuth related routes
#[expect(unused_variables)]
mod oauth;
/// The route to renew an access token
mod renew_access_token;
/// The route to resend an OTP for verification
mod resend_otp;
/// The route to reset a user's password
mod reset_password;

use self::{
	complete_sign_up::*,
	create_account::*,
	docker_login::*,
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
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.merge(oauth::setup_routes(state, allowed_client_types).await)
		.mount_endpoint(login, state, allowed_client_types)
		.mount_auth_endpoint(logout, state, allowed_client_types)
		.mount_endpoint(create_account, state, allowed_client_types)
		.mount_endpoint(renew_access_token, state, allowed_client_types)
		.mount_endpoint(forgot_password, state, allowed_client_types)
		.mount_endpoint(is_email_valid, state, allowed_client_types)
		.mount_endpoint(is_username_valid, state, allowed_client_types)
		.mount_endpoint(complete_sign_up, state, allowed_client_types)
		.mount_endpoint(list_recovery_options, state, allowed_client_types)
		.mount_endpoint(resend_otp, state, allowed_client_types)
		.mount_endpoint(reset_password, state, allowed_client_types)
		.mount_endpoint(docker_login, state, allowed_client_types)
}
