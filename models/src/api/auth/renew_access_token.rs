use crate::prelude::*;

macros::declare_api_endpoint!(
	/// This endpoint is used to get a new access token for a user. This is used
	/// when the access token expires, and requires the refresh token to be provided.
	RenewAccessToken,
	GET "/auth/access-token",
	api = false,
	request_headers = {
		/// The refresh token which was provided to the user when they logged in
		pub refresh_token: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	response = {
		/// The new access token which will be used for authentication by the user
		pub access_token: String,
	},
);
