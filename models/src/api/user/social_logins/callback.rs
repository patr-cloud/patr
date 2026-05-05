use crate::{api::auth::SocialLoginProvider, prelude::*};

macros::declare_api_endpoint!(
	/// Completes the "Connect Social Login" flow. The caller's session must match
	/// the `user_id` baked into the state token at initiate time — this
	/// prevents a connect started on account A from completing against
	/// account B (e.g. if the user logged out and logged back in as someone
	/// else between initiate and callback).
	ConnectSocialLoginCallback,
	POST "/user/social-login/{provider}/callback" {
		/// The social-login provider to connect. Must be `github` for this
		/// endpoint.
		pub provider: SocialLoginProvider,
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
	request = {
		/// The authorization code returned by GitHub in the redirect URL.
		#[preprocess(trim, length(min = 1))]
		pub code: String,
		/// The CSRF state parameter returned by GitHub — must match what was
		/// stored in Redis during the initiation step.
		#[preprocess(trim, length(min = 1))]
		pub state: String,
	},
	audit_log = NoAuditLogger,
);
