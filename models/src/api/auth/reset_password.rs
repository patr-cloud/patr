use crate::prelude::*;

macros::declare_api_endpoint!(
	// Reset passsword
	ResetPassword,
	POST "/auth/reset-password",
	request = {
		pub user_id: String,
		pub verification_token: String,
		pub password: String,
	},
);
