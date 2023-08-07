use crate::prelude::*;

macros::declare_api_endpoint!(
	Login,
	POST "/auth/sign-in",
	request = {
		pub user_id: String,
		pub password: String,
		pub mfa_otp: Option<String>,
	},
	response = {
		pub access_token: String,
		pub refresh_token: Uuid,
		pub login_id: Uuid,
	}
);
