use std::{fmt, path::PathBuf, str::FromStr};

use config::ConfigError;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// A release channel the CLI can track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "camelCase")]
#[value(rename_all = "kebab-case")]
pub enum Channel {
	/// Tracks `master` — stable releases.
	Stable,
	/// Tracks `staging` — pre-release builds, ahead of stable.
	Beta,
	/// Tracks `develop` — bleeding edge, updated on every commit.
	Alpha,
}

impl Channel {
	/// The channel this binary was built on. Falls back to [`Channel::Alpha`]
	/// for local/dev builds where CI hasn't set `PATR_BUILD_CHANNEL`.
	pub const BUILD: Self = {
		match option_env!("PATR_BUILD_CHANNEL") {
			Some(s) => match s.as_bytes() {
				b"stable" => Self::Stable,
				b"beta" => Self::Beta,
				_ => Self::Alpha,
			},
			None => Self::Alpha,
		}
	};
}

impl Default for Channel {
	fn default() -> Self {
		Self::BUILD
	}
}

impl fmt::Display for Channel {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Stable => "stable",
			Self::Beta => "beta",
			Self::Alpha => "alpha",
		})
	}
}

/// Auth portion of the CLI state. Kept as an enum so that
/// `current_workspace` can't exist without a `token`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum AuthState {
	/// The user is logged in with an API token and (optionally) a selected
	/// workspace.
	#[serde(rename_all = "camelCase")]
	LoggedIn {
		/// The user's access token.
		token: BearerToken,
		/// The currently selected workspace id.
		current_workspace: Option<Uuid>,
	},
	/// The user is logged out.
	#[default]
	LoggedOut,
}

/// State and stored data of the CLI. Written to a single `config.json` whether
/// or not the user is logged in; the `target_channel` field persists across
/// login/logout.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
	/// Release channel that `patr upgrade` tracks. Defaults to the channel
	/// the binary was built on.
	#[serde(default)]
	pub target_channel: Channel,
	/// Auth state — logged in with a token, or logged out.
	#[serde(flatten, default)]
	pub auth: AuthState,
}

impl AppState {
	/// Load the state from the config file. If the config file does not exist,
	/// return the default state.
	///
	/// The config file is loaded from the following locations in order:
	/// - The environment variable `CONFIG_PATH` if it is set
	/// - The user specific config location independent of the current platform
	/// - The system wide config location independent of the current platform
	pub fn load() -> Result<Self, AppError> {
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
		.build()
		.map_err(AppError::ConfigReadError)?
		.try_deserialize()
		.map_err(AppError::ConfigReadError)
	}

	/// Save the state to the config file. If the config file does not exist, it
	/// will be created.
	pub fn save(self) -> Result<(), AppError> {
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
		std::fs::create_dir_all(config_dir.parent().expect("Failed to get parent directory"))
			.map_err(|err| AppError::ConfigWriteError(ConfigError::Message(err.to_string())))?;
		std::fs::write(
			config_dir,
			serde_json::to_vec(&self).expect("Failed to serialize the CLI state"),
		)
		.map_err(|err| AppError::ConfigWriteError(ConfigError::Message(err.to_string())))
	}

	/// Returns true if the user is logged in, false otherwise.
	pub fn is_logged_in(&self) -> bool {
		matches!(self.auth, AuthState::LoggedIn { .. })
	}

	/// Returns true if the user is logged out, false otherwise.
	pub fn is_logged_out(&self) -> bool {
		matches!(self.auth, AuthState::LoggedOut)
	}
}
