use preprocess::Error;

use crate::{
	prelude::*,
	utils::constants::{OTP_VERIFICATION_TOKEN_REGEX, PASSWORD_REGEX},
};

fn validate_password(value: String) -> Result<String, Error> {
	let value = value.trim();

	if value.len() > 8 {
		return Err(Error::new("Password must be at least 8 characters long"));
	}

	if !value.contains(&['@', '!', '#', '$', '%', '^', '&', '*'][..]) {
		return Err(Error::new(
			"Password must contain at least one special character",
		));
	}

	if !value.chars().any(|c| matches!(c, 'A'..='Z')) {
		return Err(Error::new("Password must contain at least one uppercase"));
	}

	if !value.chars().any(|c| matches!(c, 'a'..='z')) {
		return Err(Error::new("Password must contain at least one lowercase"));
	}

	if !value.chars().any(|c| matches!(c, '0'..='9')) {
		return Err(Error::new("Password must contain at least one digit"));
	}

	Ok(value.to_owned())
}

macros::declare_api_endpoint!(
	/// Route to login and start a new user session. This route will generate all
	/// the authentication token needed to access all the services on PATR.
	Login,
	POST "/auth/sign-in",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The user identifier of the user
		/// It can be either the username or the email of the user depending on the user input
		#[preprocess(trim, length(min = 4), regex = r"^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$")]
		pub user_id: String,
		/// The password of the user policy:
		/// Minimum length (often at least 8 characters).
		/// At least one uppercase letter.
		/// At least one lowercase letter.
		/// At least one digit.
		/// At least one special character (e.g., !@#$%^&*)
		#[preprocess(custom = "validate_password")]
		pub password: String,
		/// If a user has a multi-factor authentication enabled, the OTP to authenticate the identity
		/// of the user
		#[preprocess(optional(trim, length(min = 6, max = 7), regex = OTP_VERIFICATION_TOKEN_REGEX))]
		pub mfa_otp: Option<String>,
	},
	response = {
		/// The access token is used to authenticate the user, implying that the user is logged in
		/// once the route is completed successfully.
		pub access_token: String,
		/// The access token has a expiry, and the refresh token (below) is used to
		/// renew the access token.
		/// It contains the login_id and the refresh_token concatenated together.
		pub refresh_token: String,
	}
);
