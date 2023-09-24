use models::utils::Uuid;
use serde::{Deserialize, Serialize};

/// The data that will be stored in the user's browser storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AppStorage {
	/// Storage when the user is logged in. Contains the user's ID, access
	/// token, refresh token, and default workspace.
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		/// The user's userId.
		user_id: Uuid,
		/// The user's access token.
		access_token: String,
		/// The user's refresh token.
		refresh_token: String,
		/// The user's default workspace, if they have any
		default_workspace: Option<Uuid>,
	},
	/// Storage when the user is logged out.
	#[serde(rename_all = "camelCase")]
	#[default]
	LoggedOut,
}

impl AppStorage {
	/// Returns true if the user is logged in, and false otherwise.
	pub fn is_logged_in(&self) -> bool {
		matches!(self, AppStorage::LoggedIn { .. })
	}
}
