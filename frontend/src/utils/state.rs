use models::utils::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AppStorage {
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		user_id: Uuid,
		access_token: String,
		refresh_token: String,
		default_workspace: Option<Uuid>,
	},
	#[serde(rename_all = "camelCase")]
	#[default]
	LoggedOut,
}

impl AppStorage {
	pub fn is_logged_in(&self) -> bool {
		matches!(self, AppStorage::LoggedIn { .. })
	}
}
