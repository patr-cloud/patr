use crate::prelude::*;

macros::declare_api_endpoint!(
	CompleteSignUp,
	POST "/auth/join",
	request = {
		pub username: String,
		pub verification_token: String,
	},
	response = {
		pub access_token: String,
		pub refresh_token: Uuid,
		pub login_id: Uuid,
	}
);
