use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route when user forgets their password and raises a password change request.
	/// This will send an OTP to their email address.
	ForgotPassword,
	POST "/auth/forgot-password",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The email address of the user requesting the password reset
		#[preprocess(trim, email)]
		pub email: String,
		/// The Cloudflare Turnstile token to verify that the request is made by a human
		#[preprocess(trim, length(min = 1))]
		pub cf_turnstile_token: String,
	},
	audit_log = NoAuditLogger,
);
