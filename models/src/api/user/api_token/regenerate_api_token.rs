use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Regenerate an API token. This will invalidate the old token. This can be used in case the
	/// token is compromised, or if the user wants to rotate their tokens. A new token will be
	/// generated, and sent in the response.
	RegenerateApiToken,
	POST "/user/api-token/:token_id/regenerate" {
		/// The ID of the token to regenerate
		pub token_id: Uuid,
	},
	api = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	response = {
		/// The new token
		pub token: String,
	}
);
