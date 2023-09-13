use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	// Definition of a route which renews an access token
	RenewAccessToken,
	GET "/auth/access-token",
	request_headers = {
		// The login ID to identify a user session
		pub login_id: Uuid,
	},
	query = {
		// The refresh token to get a new access token
		#[serde(skip)]
		pub refresh_token: Uuid,
	}
	response = {
		// The new access token which will be used for authentication by the user
		access_token: String,
	},
);
