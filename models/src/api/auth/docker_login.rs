use headers::{Authorization, authorization::Basic};

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to login and start a new user session. This route will generate all
	/// the authentication token needed to access all the services on PATR.
	DockerLogin,
	GET "/auth/docker-login",
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
		/// The credentials provided in the Authorization header
		pub authorization: Authorization<Basic>,
	},
	query = {
		/// The service requesting the login
		#[preprocess(trim)]
		pub service: String,
	},
	response = {
		/// The access token generated for the user
		pub access_token: String,
		/// The token (alias for access token) generated for the user
		pub token: String,
	},
);
