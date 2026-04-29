use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Initiates the GitHub OAuth2 flow. Generates a CSRF state token, stores
	/// it in Redis for 10 minutes, and returns the full GitHub authorization
	/// URL that the frontend should redirect the browser to.
	GithubOAuthInitiate,
	GET "/auth/social-login/github",
	api = false,
	response = {
		/// The full GitHub authorization URL. The frontend must redirect the
		/// user's browser to this URL to begin the OAuth flow.
		pub authorize_url: String,
	},
	audit_log = NoAuditLogger,
);
