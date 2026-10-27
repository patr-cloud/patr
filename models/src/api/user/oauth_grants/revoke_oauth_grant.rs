use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Revoke a grant, cutting the app off. Its access token stops working
	/// on its next request.
	RevokeOAuthGrant,
	DELETE "/user/oauth-grant/{grant_id}" {
		/// The grant to revoke
		pub grant_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	client_type = [WebLogin],
	audit_log = NoAuditLogger,
);
