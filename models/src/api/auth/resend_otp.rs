use crate::prelude::*;

macros::declare_api_endpoint!(
	// Resent OTP
	ResendOtp,
	POST "/auth/resend-otp",
	request = {
		pub username: String,
		pub password: String,
	},
);
