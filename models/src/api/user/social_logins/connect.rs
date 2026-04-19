use crate::{api::auth::SocialLoginProvider, prelude::*};

macros::declare_api_endpoint!(
	/// Initiates the "Connect GitHub" flow for an already-logged-in user.
	/// Mints a CSRF state token tied to the caller's `user_id` (stored in
	/// Redis for 10 minutes) and returns the GitHub authorization URL the
	/// frontend should redirect the browser to.
	ConnectSocialLoginInitiate,
	POST "/user/social-login/{provider}/connect" {
		/// The social-login provider to connect. Must be `github` for this
		/// endpoint.
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
	response = {
		/// The full GitHub authorization URL. The frontend must redirect the
		/// user's browser to this URL to begin the OAuth flow.
		pub authorize_url: String,
	},
	audit_log = NoAuditLogger,
);
