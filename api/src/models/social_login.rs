use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Lazily initialised HTTP client reused across GitHub API calls. GitHub
/// requires a `User-Agent` header on every request.
pub static GITHUB_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
	reqwest::Client::builder()
		.user_agent("patr-api/1.0")
		.build()
		.expect("failed to build GitHub HTTP client")
});

/// Payload stored in Redis between the GitHub OAuth callback and the
/// account-setup form. Carries the GitHub identity that the new Patr account
/// will be linked to once setup completes.
#[derive(Serialize, Deserialize)]
pub struct GithubSetupPayload {
	/// Stable GitHub user ID (numeric, stringified).
	pub external_id: String,
	/// Verified primary email returned by GitHub. Becomes the new account's
	/// recovery email.
	pub email: String,
}

/// Payload stored in Redis for a GitHub OAuth CSRF state token. The variant
/// tells the callback which flow this token belongs to. One Redis namespace
/// (`socialLogin:github:state:<token>`) covers both flows; the discriminator
/// lives in the JSON.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GithubStatePayload {
	/// Unauthenticated sign-in flow from `/login` or `/sign-up`. No
	/// per-user payload — the eventual identity is whatever GitHub returns.
	Anonymous,
	/// Authenticated "Connect GitHub" flow from Profile → Connected
	/// Accounts. Records which Patr user initiated the connect — checked
	/// against the caller's JWT in the callback so a connect started on one
	/// account can't complete against another.
	Authenticated {
		/// The Patr user that initiated the connect.
		user_id: Uuid,
	},
}
