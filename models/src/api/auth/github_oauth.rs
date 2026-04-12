use serde::{Deserialize, Serialize};

use crate::{prelude::*, utils::constants::USERNAME_VALIDITY_REGEX};

/// Identifies the third-party OAuth provider used to authenticate.
/// Add new variants here when additional providers (Google, Apple, …) are
/// integrated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum OAuthProvider {
	/// Github OAuth Provider.
	Github,
}

impl OAuthProvider {
	/// Returns the lowercase string stored in the database `provider` column.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Github => "github",
		}
	}
}

/// Discriminates which flow the GitHub OAuth callback result belongs to.
/// The frontend switches on this value to determine next steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum GithubCallbackStatus {
	/// Existing GitHub link found — access/refresh tokens returned, log in
	/// immediately
	LoggedIn,
	/// GitHub email matches an existing Patr account — user must confirm
	/// linking
	LinkRequired,
	/// No existing account found — user must complete profile setup
	SetupRequired,
}

macros::declare_api_endpoint!(
	/// Initiates the GitHub OAuth2 flow. Generates a CSRF state token, stores
	/// it in Redis for 10 minutes, and returns the full GitHub authorization
	/// URL that the frontend should redirect the browser to.
	GithubOAuthInitiate,
	GET "/auth/github",
	api = false,
	response = {
		/// The full GitHub authorization URL. The frontend must redirect the
		/// user's browser to this URL to begin the OAuth flow.
		pub authorize_url: String,
	},
	audit_log = NoAuditLogger,
);

macros::declare_api_endpoint!(
	/// Completes the GitHub OAuth2 flow. Verifies the CSRF state, exchanges
	/// the authorization code for a GitHub access token, fetches the user's
	/// GitHub profile, and resolves which of the three paths to take:
	/// - LoggedIn: existing GitHub link found, tokens returned
	/// - LinkRequired: GitHub email matches an existing Patr account
	/// - SetupRequired: no existing account, user must complete profile
	GithubOAuthCallback,
	POST "/auth/github/callback",
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
		/// Present when status is LinkRequired — pass to POST /auth/github/link
		pub link_token: Option<String>,
		/// Present when status is SetupRequired — pass to POST /auth/github/setup
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

macros::declare_api_endpoint!(
	/// Confirms linking a GitHub account to an existing Patr account. The
	/// link_token was returned by the callback endpoint and is stored in Redis
	/// for 5 minutes. Consuming it is one-time-use. Returns Patr tokens.
	GithubOAuthLink,
	POST "/auth/github/link",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The link token returned by POST /auth/github/callback
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

macros::declare_api_endpoint!(
	/// Creates a new Patr account from a GitHub identity after the user has
	/// confirmed/edited the pre-filled profile details. The setup_token was
	/// returned by the callback endpoint. Returns Patr tokens on success.
	GithubOAuthSetup,
	POST "/auth/github/setup",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The setup token returned by POST /auth/github/callback
		#[preprocess(trim, length(min = 1))]
		pub setup_token: String,
		/// The chosen Patr username
		#[preprocess(trim, length(min = 2), regex = USERNAME_VALIDITY_REGEX)]
		pub username: String,
		/// The user's first name
		#[preprocess(trim, length(min = 1))]
		pub first_name: String,
		/// The user's last name
		#[preprocess(trim, length(min = 1))]
		pub last_name: String,
	},
	response = {
		/// Patr JWT access token
		pub access_token: String,
		/// Patr refresh token
		pub refresh_token: String,
	},
	audit_log = NoAuditLogger,
);
