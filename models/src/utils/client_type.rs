use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// The type of client used for a request. This determines which authentication
/// method to use and which endpoints are accessible.
///
/// - [`WebDashboard`][Self::WebDashboard]: Requests from the web dashboard,
///   authenticated via JWT.
/// - [`ApiToken`][Self::ApiToken]: Requests from third-party applications,
///   authenticated via user API tokens (`patrv1.*`).
/// - [`ServiceAccount`][Self::ServiceAccount]: Requests from service accounts
///   (non-human identities like runners), authenticated via service account
///   tokens (`patrv1.*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientType {
	/// The request is authenticated using a JWT from the web dashboard
	WebDashboard,
	/// The request is authenticated using a user API token
	ApiToken,
	/// The request is authenticated using a service account token
	ServiceAccount,
}

impl Display for ClientType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::WebDashboard => write!(f, "WebDashboard"),
			Self::ApiToken => write!(f, "ApiToken"),
			Self::ServiceAccount => write!(f, "ServiceAccount"),
		}
	}
}
