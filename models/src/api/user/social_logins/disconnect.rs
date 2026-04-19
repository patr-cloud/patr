use crate::{api::auth::SocialLoginProvider, prelude::*};

macros::declare_api_endpoint!(
	/// Disconnect a social-login provider from the caller's Patr account. The
	/// underlying account remains; only the OAuth link is removed. The user
	/// can sign in again with their password (or via password reset on their
	/// recovery email) afterwards.
	DisconnectSocialLogin,
	DELETE "/user/social-login/{provider}" {
		/// The provider to disconnect.
		pub provider: SocialLoginProvider,
	},
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
	audit_log = NoAuditLogger,
);
