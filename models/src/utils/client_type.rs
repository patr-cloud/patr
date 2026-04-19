use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// Which kind of credential a request authenticated with. This decides which
/// parser handles the token and which endpoints the caller may reach.
///
/// The database keeps this on two levels — `ACTOR_CLIENT_TYPE` separates a
/// user login from a service account, and `USER_LOGIN_TYPE` beneath it
/// separates a web session from an API token. This flattens both, because an
/// endpoint's allowlist has to tell all three apart.
///
/// - [`WebDashboard`][Self::WebDashboard]: Requests from the web dashboard, authenticated via JWT.
/// - [`ApiToken`][Self::ApiToken]: Requests from third-party applications, authenticated via user
///   API tokens (`patrv1.*`).
/// - [`ServiceAccount`][Self::ServiceAccount]: Requests from service accounts (non-human identities
///   like runners), authenticated via service account tokens (`patrv1.*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActorClientType {
	/// The request is authenticated using a JWT from the web dashboard
	WebDashboard,
	/// The request is authenticated using a user API token
	ApiToken,
	/// The request is authenticated using a service account token
	ServiceAccount,
}

impl Display for ActorClientType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::WebDashboard => write!(f, "WebDashboard"),
			Self::ApiToken => write!(f, "ApiToken"),
			Self::ServiceAccount => write!(f, "ServiceAccount"),
		}
	}
}
