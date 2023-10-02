use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Set the information of the currently authenticated user.
	UpdateUserInfo,
	POST "/user",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	request = {
		/// The first name of the user.
		pub first_name: Option<String>,
		/// The last name of the user.
		pub last_name: Option<String>,
		// TODO MFA stuff
	},
);
