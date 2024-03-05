use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Revoke an API token. This will invalidate the token, and it will no longer be usable.
	RevokeApiToken,
	DELETE "/user/api-token/:token_id" {
		/// The ID of the token to revoke
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
);
