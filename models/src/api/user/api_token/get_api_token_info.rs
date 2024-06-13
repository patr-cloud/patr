use super::UserApiToken;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// List all API tokens for a particular user.
	GetApiTokenInfo,
	GET "/user/api-token/:token_id" {
		/// The ID of the API token to retrieve
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
		/// The token to create
		#[serde(flatten)]
		pub token: WithId<UserApiToken>,
	}
);
