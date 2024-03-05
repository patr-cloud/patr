use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to login and start a new user session. This route will generate all
	/// the authentication token needed to access all the services on PATR.
	Login,
	POST "/auth/sign-in",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The user identifier of the user
		/// It can be either the username or the email of the user depending on the user input
		pub user_id: String,
		/// The password of the user
		pub password: String,
		/// If a user has a multi-factor authentication enabled, the OTP to authenticate the identity
		/// of the user
		pub mfa_otp: Option<String>,
	},
	response = {
		/// The access token is used to authenticate the user, implying that the user is logged in
		/// once the route is completed successfully.
		pub access_token: String,
		/// The access token has a expiry, and the refresh token (below) is used to
		/// renew the access token.
		/// It contains the login_id and the refresh_token concatenated together.
		pub refresh_token: String,
	}
);
