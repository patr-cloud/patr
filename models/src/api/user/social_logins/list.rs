use super::LinkedSocialLogin;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// List the social-login providers (GitHub, …) currently linked to the
	/// caller's Patr account.
	ListSocialLogins,
	GET "/user/social-login",
	client_type = [WebDashboard],
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
		/// Linked providers, ordered by `linked_at` ascending.
		pub logins: Vec<LinkedSocialLogin>,
	},
	audit_log = NoAuditLogger,
);
