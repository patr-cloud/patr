use super::GithubCallbackStatus;
use crate::{api::auth::SocialLoginProvider, prelude::*};

macros::declare_api_endpoint!(
	/// Completes the social-login OAuth flow. Verifies the CSRF state,
	/// exchanges the authorization code for a provider access token, fetches
	/// the user profile, and resolves which path to take:
	/// - LoggedIn: existing link found (or auto-linked via verified email),
	///   tokens returned
	/// - SetupRequired: no existing account, user must complete profile
	SocialLoginCallback,
	POST "/auth/social-login/{provider}/callback" {
		/// The social-login provider this callback belongs to. Must be
		/// `github` for now.
		pub provider: SocialLoginProvider,
	},
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The authorization code returned by the provider in the redirect
		/// URL
		#[preprocess(trim, length(min = 1))]
		pub code: String,
		/// The CSRF state parameter returned by the provider — must match
		/// what was stored in Redis during the initiation step
		#[preprocess(trim, length(min = 1))]
		pub state: String,
	},
	response = {
		/// Tagged on `status` — one of `loggedIn`, `setupRequired`.
		/// Flattened so the variant fields appear at the top level of the
		/// response body.
		#[serde(flatten)]
		pub status: GithubCallbackStatus,
	},
	audit_log = NoAuditLogger,
);
