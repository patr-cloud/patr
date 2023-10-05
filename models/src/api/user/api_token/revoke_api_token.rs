use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Revoke an API token. This will invalidate the token, and it will no longer be usable.
	RevokeApiToken,
	DELETE "/user/api-token/:token_id" {
		/// The ID of the token to revoke
		pub token_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
);
