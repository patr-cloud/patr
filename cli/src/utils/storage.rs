use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

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
		#[serde(rename = "refreshtoken")]
		refresh_token: String,
		/// The current workspace that is selected by the user
		#[serde(rename = "currentworkspace")]
		current_workspace: Option<Uuid>,
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
		if let Ok(config_path) = std::env::var("CONFIG_PATH") {
			config::Config::builder()
				.add_source(config::File::with_name(&config_path).required(false))
		} else if cfg!(debug_assertions) {
			config::Config::builder().add_source(
				config::File::with_name(concat!(env!("CARGO_MANIFEST_DIR"), "/../config/cli.json"))
					.required(false),
			)
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
		.inspect_err(|err| {
			eprintln!("{}", err);
		})
		.context("Failed to deserialize the CLI state")
	}

	/// Save the state to the config file. If the config file does not exist, it
	/// will be created.
	pub fn save(self) -> Result<(), std::io::Error> {
		let config_dir = PathBuf::from_str(
			std::env::var("CONFIG_PATH").ok().as_deref().unwrap_or(
				if cfg!(debug_assertions) {
					concat!(env!("CARGO_MANIFEST_DIR"), "/../config/cli.json").to_string()
				} else {
					crate::utils::config_local_dir()
						.to_string_lossy()
						.to_string()
				}
				.as_str(),
			),
		)
		.unwrap();
		std::fs::create_dir_all(config_dir.parent().unwrap())?;
		std::fs::write(
			config_dir,
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
