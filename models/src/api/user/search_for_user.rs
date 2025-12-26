use super::BasicUserInfo;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get a user's details by userId. This will return the user's basic info,
	/// such as their username, first name, last name, etc.
	SearchForUser,
	GET "/user/search",
	api = false,
	query = {
		/// The search query to find users by username, first name, or last name.
		pub query: String,
	},
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
		/// The list of users matching the search query.
		pub users: Vec<WithId<BasicUserInfo>>,
	}
);
