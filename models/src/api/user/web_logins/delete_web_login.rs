use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Delete a user login. This will automatically expire the login session
	/// and log the user out.
	DeleteWebLogin,
	DELETE "/user/login/:login_id" {
		/// The login ID to delete.
		pub login_id: Uuid,
	},
	api = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	}
);
