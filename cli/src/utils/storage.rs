use anyhow::Context;
use models::utils::BearerToken;
use serde::{Deserialize, Serialize};

/// State and stored data of the CLI
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum AppState {
	/// The state of the CLI when the user is logged in
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		/// The user's access token
		token: BearerToken,
		/// The user's refresh token
		refresh_token: String,
	},
	/// The state of the CLI when the user is logged out
	#[serde(rename_all = "camelCase")]
	#[default]
	LoggedOut,
}

impl AppState {
	/// Load the state from the config file. If the config file does not exist,
	/// return the default state.
	///
	/// The config file is loaded from the following locations in order:
	/// - The environment variable `CONFIG_PATH` if it is set
	/// - The user specific config location independent of the current platform
	/// - The system wide config location independent of the current platform
	pub fn load() -> Result<Self, anyhow::Error> {
		if let Some(config_path) = std::env::var("CONFIG_PATH").ok() {
			config::Config::builder()
				.add_source(config::File::with_name(&config_path).required(false))
		} else {
			config::Config::builder()
		}
		.add_source(
			config::File::with_name(&crate::utils::config_dir().to_string_lossy()).required(false),
		)
		.add_source(
			config::File::with_name(&crate::utils::config_local_dir().to_string_lossy())
				.required(false),
		)
		.build()?
		.try_deserialize()
		.context("Failed to deserialize the CLI state")
	}

	/// Save the state to the config file. If the config file does not exist, it
	/// will be created.
	pub fn save(self) -> Result<(), std::io::Error> {
		std::fs::write(
			std::env::var("CONFIG_PATH")
				.ok()
				.as_deref()
				.unwrap_or(&crate::utils::config_local_dir().to_string_lossy()),
			serde_json::to_vec(&self).expect("Failed to serialize the CLI state"),
		)
	}

	/// Returns true if the user is logged in, false otherwise.
	pub fn is_logged_in(&self) -> bool {
		matches!(self, Self::LoggedIn { .. })
	}

	/// Returns true if the user is logged out, false otherwise.
	pub fn is_logged_out(&self) -> bool {
		matches!(self, Self::LoggedOut)
	}
}
