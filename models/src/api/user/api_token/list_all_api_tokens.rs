use super::UserApiToken;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// List all API tokens for a particular user.
	ListApiTokens,
	GET "/user/api-token",
	pagination = true,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	response = {
		/// The list of API tokens
		pub tokens: Vec<WithId<UserApiToken>>,
	}
);
