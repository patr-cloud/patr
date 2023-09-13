use crate::prelude::*;

macros::declare_api_endpoint!(
	// Definition of a route to validate user's entered username is available or not
	IsUsernameValid,
	GET "/auth/username-valid",
	query = {
		// The username that has to be verified
		pub username: String,
	},
	response = {
		// A boolean response corresponding the availability of the username
		pub available: bool,
	}
);
