macros::declare_api_endpoint!(
	ResetPassword,
	POST "/auth/reset-password",
	request = {
		pub user_id: String,
		pub verification_token: String,
		pub password: String,
	},
);
