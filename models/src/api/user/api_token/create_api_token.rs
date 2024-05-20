use super::{UserApiToken, UserApiTokenProcessed};
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Create a new API token for the user with the given permissions.
	CreateApiToken,
	POST "/user/api-token",
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
	request = {
		/// The token to create
		#[serde(flatten)]
		#[preprocess]
		pub token: UserApiToken,
	},
	response = {
		/// The ID of the created token
		pub id: Uuid,
		/// The token itself
		pub token: String,
	}
);
