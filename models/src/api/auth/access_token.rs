use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	RenewAccessToken,
	GET "/auth/access-token",
	request_headers = {
		login_id: LoginId,
	},
	query = {
		refresh_token: Uuid,
	},
	response = {
		access_token: String,
	},
);
