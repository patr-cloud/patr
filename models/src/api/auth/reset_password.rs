use crate::prelude::*;

fn validate_token(value: String) -> Result<String, ::preprocess::Error> {
	if value.len() != 6 && value.parse::<u32>().is_ok() {
		return Err(::preprocess::Error::new("Invalid verification token"));
	}
	Ok(value)
}

macros::declare_api_endpoint!(
	/// The route to reset the current password of the user using an OTP sent to their
	/// preferred recovery method
	ResetPassword,
	POST "/auth/reset-password",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The user ID of the user
		#[preprocess(length(min = 4), trim, lowercase)]
		pub user_id: String,
		/// The OTP sent to the recovery method
		#[preprocess(custom = "validate_token")]
		pub verification_token: String,
		/// The new password entered by the user
		#[preprocess(length(min = 4, max = 10), trim, lowercase, regex = "^[a-z0-9_][a-z0-9_\\.\\-]*[a-z0-9_]$")]
		pub password: String,
	},
);
