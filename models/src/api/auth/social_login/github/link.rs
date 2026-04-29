use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Confirms linking a GitHub account to an existing Patr account. The
	/// link_token was returned by the callback endpoint and is stored in Redis
	/// for 5 minutes. Consuming it is one-time-use. Returns Patr tokens.
	GithubOAuthLink,
	POST "/auth/social-login/github/link",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The link token returned by POST /auth/social-login/github/callback
		#[preprocess(trim, length(min = 1))]
		pub link_token: String,
	},
	response = {
		/// Patr JWT access token
		pub access_token: String,
		/// Patr refresh token
		pub refresh_token: String,
	},
	audit_log = NoAuditLogger,
);
