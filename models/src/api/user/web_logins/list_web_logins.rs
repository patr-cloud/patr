use super::UserWebLogin;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// List all web logins for the current user.
	ListWebLogins,
	GET "/user/login",
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
		/// The list of logins for the user
		pub logins: Vec<WithId<UserWebLogin>>,
	}
);
