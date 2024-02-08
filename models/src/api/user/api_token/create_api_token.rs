use super::UserApiToken;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Create a new API token for the user with the given permissions.
	CreateApiToken,
	POST "/user/api-token",
	api = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// The token to create
		#[serde(flatten)]
		pub token: UserApiToken,
	},
	response = {
		/// The ID of the created token
		pub id: Uuid,
		/// The token itself
		pub token: String,
	}
);
