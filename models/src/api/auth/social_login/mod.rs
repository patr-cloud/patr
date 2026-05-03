use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// GitHub OAuth2 SSO endpoints
mod github;

pub use self::github::*;

/// Identifies the third-party OAuth provider used to authenticate.
/// Add new variants here when additional providers (Google, Apple, …) are
/// integrated. The `Display` impl returns the lowercase token used in the
/// database `provider` column and in Redis key namespaces.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum OAuthProvider {
	/// Github OAuth Provider.
	Github,
}

impl Display for OAuthProvider {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Github => "github",
			}
		)
	}
}
