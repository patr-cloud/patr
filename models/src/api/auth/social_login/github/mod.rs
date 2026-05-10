use serde::{Deserialize, Serialize};

/// Endpoint to handle the GitHub OAuth2 callback
mod callback;
/// Endpoint to initiate the GitHub OAuth2 flow
mod initiate;
/// Endpoint to complete the GitHub OAuth2 sign up flow
mod setup;

pub use self::{callback::*, initiate::*, setup::*};

/// Result of the GitHub OAuth2 callback. Tagged on `status`; each variant
/// carries exactly the fields needed for that branch — no optionals.
///
/// If the GitHub account is already linked **or** the verified GitHub email
/// matches an existing Patr account, the callback logs the user in and binds
/// the link in one step. New users are sent through the setup form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "status", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum GithubCallbackStatus {
	/// Existing link or auto-linked via verified email match — log in
	/// immediately with the returned tokens.
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		/// Patr JWT access token
		access_token: String,
		/// Patr refresh token (`{login_id}.{refresh_token}`)
		refresh_token: String,
	},
	/// No existing account found — user must complete profile setup via
	/// `POST /auth/social-login/github/setup`.
	#[serde(rename_all = "camelCase")]
	SetupRequired {
		/// One-time-use setup token
		setup_token: String,
		/// Pre-filled first name from GitHub display name (editable; empty if
		/// GitHub had no display name)
		prefilled_first_name: String,
		/// Pre-filled last name from GitHub display name (editable; empty if
		/// GitHub had no display name)
		prefilled_last_name: String,
		/// Pre-filled email from GitHub primary verified email
		prefilled_email: String,
	},
}
