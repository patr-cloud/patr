use crate::{
	prelude::*,
	utils::{constants::USER_NAME_REGEX, validate_password},
};

macros::declare_api_endpoint!(
	/// The route to create a new user account
	CreateAccount,
	POST "/auth/sign-up",
	client_type = [WebDashboard],
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The email address of the user signing up. This is their unique
		/// identifier, and where their verification OTP is sent.
		#[preprocess(trim, email)]
		pub email: String,
		/// The password policy:
		/// Minimum length (often at least 8 characters).
		/// At least one uppercase letter.
		/// At least one lowercase letter.
		/// At least one digit.
		/// At least one special character (e.g., !@#$%^&*)
		#[preprocess(trim, length(min = 8), custom = "validate_password")]
		pub password: String,
		/// The first name of the user
		#[preprocess(trim, regex = USER_NAME_REGEX)]
		pub first_name: String,
		/// The last name of the user
		#[preprocess(trim, regex = USER_NAME_REGEX)]
		pub last_name: String,
		/// The Cloudflare Turnstile token to verify that the request is made by a human
		#[preprocess(trim, length(min = 1))]
		pub cf_turnstile_token: String,
	},
	audit_log = NoAuditLogger,
);
