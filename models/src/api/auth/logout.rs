use crate::prelude::*;

macros::declare_api_endpoint!(
	/// The route to logout a user and end the current user session by discarding the
	/// authentication token and refresh token. This will invalidate the refresh token
	/// and access token associated with it.
	Logout,
	POST "/auth/sign-out",
	api = false,
	request_headers = {
		/// The refresh token which was provided to the user when they logged in
		pub refresh_token: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
);
