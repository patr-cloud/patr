/// All OAuth related endpoints go here.
pub mod oauth;

/// The endpoint to complete the sign up process
mod complete_sign_up;
/// The endpoint to create an account (a.k.a. sign up)
mod create_account;
/// The endpoint to trigger a forgot password flow
mod forgot_password;
/// The endpoint to check if an email is valid
mod is_email_valid;
/// The endpoint to check if a username is valid
mod is_username_valid;
/// The endpoint to list the recovery options for a user
mod list_recovery_options;
/// The endpoint to login
mod login;
/// The endpoint to logout
mod logout;
/// The endpoint to renew the access token
mod renew_access_token;
/// The endpoint to resend the OTP
mod resend_otp;
/// The endpoint to reset the password
mod reset_password;

pub use self::{
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
