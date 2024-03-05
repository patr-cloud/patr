use crate::prelude::*;

macros::declare_api_endpoint!(
	/// The route to check if a user's username is available to be used to create an account or not
	IsUsernameValid,
	GET "/auth/username-valid",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	query = {
		/// The username that has to be verified
		pub username: String,
	},
	response = {
		/// A boolean response corresponding the availability of the username
		pub available: bool,
	}
);
