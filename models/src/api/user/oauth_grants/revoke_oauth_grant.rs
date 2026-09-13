use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Revoke a single grant, ending one app's access on the user's behalf.
	///
	/// Takes effect on the next request the app makes: the authenticator reads
	/// the grant row every time, so an access token that has not expired yet
	/// stops working immediately rather than lingering until it does.
	RevokeOAuthGrant,
	DELETE "/user/oauth-grant/{grant_id}" {
		/// The grant to revoke
		pub grant_id: Uuid,
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
