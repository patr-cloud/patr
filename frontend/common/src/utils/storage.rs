use serde::{Deserialize, Serialize};

/// The Type of the App, whether it is hosted or self hosted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AppType {
	/// The app is hosted and managed by patr
	Managed,
	/// The app is self hosted
	SelfHosted,
}

impl AppType {
	/// Returns true if the frontend is running on a managed instance
	pub fn is_managed(&self) -> bool {
		matches!(self, AppType::Managed)
	}

	/// Returns true if the frontend is running on a self-hosted instance
	pub fn is_self_hosted(&self) -> bool {
		matches!(self, AppType::SelfHosted)
	}
}

/// The auth state stores the information about the user's login status, along
/// with the data associated with the login, if logged in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AuthState {
	/// The user is logged out
	#[default]
	LoggedOut,
	/// The user is logged in
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		/// The JWT access token. Used to authenticate requests to the server
		/// and is stored in the browser cookies
		access_token: String,
		/// The Refresh Token, used to get a new access token when the current
		/// one expires or is invalid
		refresh_token: String,
	},
}

impl AuthState {
	/// Get the access token if the user is logged in
	pub fn get_access_token(&self) -> Option<String> {
		match self {
			AuthState::LoggedIn { access_token, .. } => Some(access_token.to_owned()),
			_ => None,
		}
	}

	/// Get the refresh token if the user is logged in
	pub fn get_refresh_token(&self) -> Option<&str> {
		match self {
			AuthState::LoggedIn { refresh_token, .. } => Some(refresh_token),
			_ => None,
		}
	}

	/// Get the access token, and panic if the user is not logged in
	pub fn expect_access_token(&self) -> &str {
		match self {
			AuthState::LoggedIn { access_token, .. } => access_token,
			_ => panic!("user is not logged in"),
		}
	}

	/// Get the refresh token, and panic if the user is not logged in
	pub fn expect_refresh_token(&self) -> &str {
		match self {
			AuthState::LoggedIn { refresh_token, .. } => refresh_token,
			_ => panic!("user is not logged in"),
		}
	}

	/// Check if the user is logged in
	pub fn is_logged_in(&self) -> bool {
		matches!(self, AuthState::LoggedIn { .. })
	}

	/// Check if the user is logged out
	pub fn is_logged_out(&self) -> bool {
		matches!(self, AuthState::LoggedOut)
	}
}
