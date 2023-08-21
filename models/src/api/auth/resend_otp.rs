macros::declare_api_endpoint!(
	ResendOtp,
	POST "/auth/resend-otp",
	request = {
		pub username: String,
		pub password: String,
	},
);
