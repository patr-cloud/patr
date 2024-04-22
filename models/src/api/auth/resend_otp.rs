use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to resent an OTP to the linked recovery method opted by the user to
	/// verify their account. The recovery method can either be an email or a phone number.
	ResendOtp,
	POST "/auth/resend-otp",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The username of the user
		#[preprocess(trim, lowercase)]
		pub username: String,
		/// The password of the user
		#[preprocess(none)]
		pub password: String,
	},
);
