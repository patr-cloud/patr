use super::UserApiToken;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// List all API tokens for a particular user.
	ListApiTokens,
	GET "/user/api-token",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	pagination = true,
	response_headers = {
		/// The total number of databases in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of API tokens
		pub tokens: Vec<WithId<UserApiToken>>,
	}
);
