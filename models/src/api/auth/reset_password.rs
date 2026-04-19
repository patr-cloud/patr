use crate::{
	prelude::*,
	utils::{constants::OTP_VERIFICATION_TOKEN_REGEX, validate_password},
};

macros::declare_api_endpoint!(
	/// The route to reset the current password of the user using an OTP sent to their
	/// preferred recovery method
	ResetPassword,
	POST "/auth/reset-password",
	client_type = [WebDashboard],
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The email address of the user resetting their password
		#[preprocess(trim, email)]
		pub email: String,
		/// The new password entered by the user
		#[preprocess(trim, length(min = 8), custom = "validate_password")]
		pub password: String,
		/// The OTP sent to the recovery method
		#[preprocess(trim, length(equal = 6), regex = OTP_VERIFICATION_TOKEN_REGEX)]
		pub verification_token: String,
		/// The Cloudflare Turnstile token to verify that the request is made by a human
		#[preprocess(trim, length(min = 1))]
		pub cf_turnstile_token: String,
	},
	audit_log = NoAuditLogger,
);
