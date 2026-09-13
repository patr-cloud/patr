use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Revoke every grant the user holds for one app.
	///
	/// A user thinks in terms of the app, not the grant — "stop Grafana using
	/// my account" should not require finding and revoking three separate
	/// sessions they did not know they had.
	RevokeOAuthGrantsForClient,
	DELETE "/user/oauth-grant/client/{client_id}" {
		/// The app to cut off
		pub client_id: String,
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
	audit_log = NoAuditLogger,
);
