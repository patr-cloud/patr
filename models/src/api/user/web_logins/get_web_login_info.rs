use super::UserWebLogin;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get information about a specific web login.
	GetWebLoginInfo,
	GET "/user/login/:login_id" {
		/// The login ID to get information about.
		pub login_id: Uuid,
	},
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
	response = {
		/// The login information.
		#[serde(flatten)]
		pub login: WithId<UserWebLogin>,
	}
);
