/// The client used to make requests to the Patr API
mod client;
/// The storage module, used to store data between CLI sessions such as the
/// user's API token or access token + refresh token
mod storage;

pub use self::{client::*, storage::*};

/// Constants used in the CLI
pub mod constants {
	/// The base URL for the Patr API
	pub const API_BASE_URL: &str = if cfg!(debug_assertions) {
		"http://localhost:3000"
	} else {
		"https://api.patr.cloud"
	};

	/// The user agent for the CLI
	pub const USER_AGENT_STRING: &str = concat!(
		"patr-cli/",
		env!("CARGO_PKG_VERSION_MAJOR"),
		".",
		env!("CARGO_PKG_VERSION_MINOR")
	);
}

/// The location for config files for the CLI
pub fn config_dir() -> std::path::PathBuf {
	dirs::data_dir()
		.expect("Failed to get the system config directory")
		.join("patr-cli")
		.join("config.json")
}

/// The location for the local config files for the CLI
pub fn config_local_dir() -> std::path::PathBuf {
	dirs::data_local_dir()
		.expect("Failed to get the user's config directory")
		.join("patr-cli")
		.join("config.json")
}

/// A trait to convert a serde type to a JSON value. This is useful for
/// serializing types that implement `serde::Serialize` to a JSON value.
pub trait ToJsonValue {
	/// Convert the type to a JSON value
	fn to_json_value(&self) -> serde_json::Value;
}

impl<T> ToJsonValue for T
where
	T: serde::Serialize,
{
	fn to_json_value(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("Failed to serialize to JSON")
	}
}
