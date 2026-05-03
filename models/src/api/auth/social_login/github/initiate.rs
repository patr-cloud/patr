use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Initiates the GitHub OAuth2 flow. Generates a CSRF state token, stores
	/// it in Redis for 10 minutes, and returns the full GitHub authorization
	/// URL that the frontend should redirect the browser to.
	GithubOAuthInitiate,
	POST "/auth/social-login/github",
	api = false,
	request = {
		/// The Cloudflare Turnstile token to verify that the request is made by
		/// a human. Reuses the page-level Turnstile widget from the login or
		/// signup page that surfaced the GitHub button.
		#[preprocess(trim, length(min = 1))]
		pub cf_turnstile_token: String,
	},
	response = {
		/// The full GitHub authorization URL. The frontend must redirect the
		/// user's browser to this URL to begin the OAuth flow.
		pub authorize_url: String,
	},
	audit_log = NoAuditLogger,
);
