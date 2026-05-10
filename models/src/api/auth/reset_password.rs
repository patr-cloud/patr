use crate::{
	prelude::*,
	utils::{constants::OTP_VERIFICATION_TOKEN_REGEX, validate_password},
};

macros::declare_api_endpoint!(
	/// The route to reset the current password of the user using an OTP sent
	/// to their email.
	ResetPassword,
	POST "/auth/reset-password",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The email address of the user
		#[preprocess(email)]
		pub email: String,
		/// The new password entered by the user
		#[preprocess(trim, length(min = 8), custom = "validate_password")]
		pub password: String,
		/// The OTP sent to the recovery email
		#[preprocess(trim, length(equal = 6), regex = OTP_VERIFICATION_TOKEN_REGEX)]
		pub verification_token: String,
	},
	audit_log = NoAuditLogger,
);
