use crate::prelude::*;

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
		pub user_id: String,
		/// The OTP sent to the recovery method
		pub verification_token: String,
		/// The new password entered by the user
		pub password: String,
	},
);
