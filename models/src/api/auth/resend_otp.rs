use crate::{
	prelude::*,
	utils::constants::{PASSWORD_REGEX, USERNAME_VALIDITY_REGEX},
};

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
		#[preprocess(trim, length(min = 2), regex = USERNAME_VALIDITY_REGEX)]
		pub username: String,
		/// The password of the user
		#[preprocess(trim, length(min = 8), regex = PASSWORD_REGEX)]
		pub password: String,
	},
);
