use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Set the information of the currently authenticated user.
	UpdateUserInfo,
	PATCH "/user",
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
		/// The first name of the user.
		#[preprocess(none)]
		pub first_name: Option<String>,
		/// The last name of the user.
		#[preprocess(none)]
		pub last_name: Option<String>,
		// TODO MFA stuff
	},
);
