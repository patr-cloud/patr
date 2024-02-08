macros::declare_api_endpoint!(
	/// Route to resent an OTP to the linked recovery method opted by the user to
	/// verify their account. The recovery method can either be an email or a phone number.
	ResendOtp,
	POST "/auth/resend-otp",
	api = false,
	request = {
		/// The username of the user
		pub username: String,
		/// The password of the user
		pub password: String,
	},
);
