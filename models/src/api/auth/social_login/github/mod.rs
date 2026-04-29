use serde::{Deserialize, Serialize};

/// Endpoint to initiate the GitHub OAuth2 flow
mod initiate;
/// Endpoint to handle the GitHub OAuth2 callback
mod callback;
/// Endpoint to confirm linking a GitHub account to an existing Patr account
mod link;
/// Endpoint to complete the GitHub OAuth2 sign up flow
mod setup;

pub use self::{callback::*, initiate::*, link::*, setup::*};

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
