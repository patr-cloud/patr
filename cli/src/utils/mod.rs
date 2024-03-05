use serde::{Deserialize, Serialize};

/// The client used to make requests to the Patr API
mod client;

pub use client::*;

/// Constants used in the CLI
pub mod constants {
	/// The base URL for the Patr API
	pub const API_BASE_URL: &str = if cfg!(debug_assertions) {
		"https://api.patr.cloud"
	} else {
		"http://localhost:3000"
	};
}

/// State and stored data of the CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppState {
	/// The state of the CLI when the user is logged in
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		/// The user's ID
		user_id: String,
		/// The user's API token
		token: String,
	},
	/// The state of the CLI when the user is logged out
	#[serde(rename_all = "camelCase")]
	LoggedOut,
}
