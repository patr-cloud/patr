macros::declare_api_endpoint!(
	/// Definition of a route to resend an OTP to the linked recovery method opted by the user to 
	/// verify their account
	ResendOtp,
	POST "/auth/resend-otp",
	request = {
		/// The username of the user
		pub username: String,
		/// The password of the user
		pub password: String,
	},
);
