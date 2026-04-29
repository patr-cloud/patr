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
		/// Which path was taken — frontend switches on this value
		pub status: GithubCallbackStatus,
		/// Present when status is LoggedIn
		pub access_token: Option<String>,
		/// Present when status is LoggedIn
		pub refresh_token: Option<String>,
		/// Present when status is LinkRequired — pass to POST /auth/social-login/github/link
		pub link_token: Option<String>,
		/// Present when status is SetupRequired — pass to POST /auth/social-login/github/setup
		pub setup_token: Option<String>,
		/// Pre-filled username suggestion from GitHub login (editable by user)
		pub prefilled_username: Option<String>,
		/// Pre-filled first name from GitHub display name (editable by user)
		pub prefilled_first_name: Option<String>,
		/// Pre-filled last name from GitHub display name (editable by user)
		pub prefilled_last_name: Option<String>,
		/// Pre-filled email from GitHub primary verified email (editable by user)
		pub prefilled_email: Option<String>,
	},
	audit_log = NoAuditLogger,
);
