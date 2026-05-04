use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// GitHub OAuth2 SSO endpoints
mod github;

pub use self::github::*;

/// Identifies the third-party OAuth provider used to authenticate.
/// Add new variants here when additional providers (Google, Apple, …) are
/// integrated. The `Display` impl returns the lowercase token used in the
/// database `provider` column and in Redis key namespaces.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, rename_all = "lowercase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	derive(sqlx::Type),
	sqlx(type_name = "SOCIAL_LOGIN_PROVIDER", rename_all = "lowercase")
)]
pub enum SocialLoginProvider {
	/// GitHub OAuth Provider.
	GitHub,
}

impl Display for SocialLoginProvider {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::GitHub => "github",
			}
		)
	}
}
