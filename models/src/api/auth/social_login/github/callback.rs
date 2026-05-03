use super::GithubCallbackStatus;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Completes the GitHub OAuth2 flow. Verifies the CSRF state, exchanges
	/// the authorization code for a GitHub access token, fetches the user's
	/// GitHub profile, and resolves which of the three paths to take:
	/// - LoggedIn: existing GitHub link found, tokens returned
	/// - LinkRequired: GitHub email matches an existing Patr account
	/// - SetupRequired: no existing account, user must complete profile
	GithubOAuthCallback,
	POST "/auth/social-login/github/callback",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The authorization code returned by GitHub in the redirect URL
		#[preprocess(trim, length(min = 1))]
		pub code: String,
		/// The CSRF state parameter returned by GitHub — must match what was
		/// stored in Redis during the initiation step
		#[preprocess(trim, length(min = 1))]
		pub state: String,
	},
	response = {
		/// Tagged on `status` — one of `loggedIn`, `linkRequired`,
		/// `setupRequired`. Flattened so the variant fields appear at the top
		/// level of the response body.
		#[serde(flatten)]
		pub status: GithubCallbackStatus,
	},
	audit_log = NoAuditLogger,
);
