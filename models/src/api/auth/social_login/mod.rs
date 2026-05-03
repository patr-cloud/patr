use serde::{Deserialize, Serialize};

/// GitHub OAuth2 SSO endpoints
mod github;

pub use self::github::*;

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
