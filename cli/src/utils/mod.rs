/// The module that contains the authentication logic for the CLI
mod authenticator;
/// The client used to make requests to the Patr API
mod client;
/// The module that contains the extension traits that are used to extend
/// functionalities to help make it easier to work with the CLI code
mod ext_trait;
/// A reusable search-and-select prompt widget for async search-based selection
pub mod search_and_select;
/// The storage module, used to store data between CLI sessions such as the
/// user's API token or access token + refresh token
mod storage;

pub use self::{authenticator::*, client::*, ext_trait::*, search_and_select::*, storage::*};

/// A list of all possible runner types that can be setup or run.
#[derive(Debug, Copy, Clone, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum RunnerType {
	/// A runner that runs on a local machine and uses Docker to run the
	/// containers
	Docker,
	/// A runner that runs on a Kubernetes cluster and uses the Kubernetes API
	/// to run the containers
	Kubernetes,
}

/// Constants used in the CLI
pub mod constants {
	use headers::UserAgent;

	/// The base URL for the Patr API
	pub const API_BASE_URL: &str = if cfg!(debug_assertions) {
		"http://localhost:3000"
	} else {
		"https://api.patr.cloud"
	};

	/// The base URL for the Patr Frontend
	pub const FRONTEND_BASE_URL: &str = if cfg!(debug_assertions) {
		"http://localhost:3001"
	} else {
		"https://app.patr.cloud"
	};

	/// The user agent for the CLI
	pub const USER_AGENT: UserAgent = UserAgent::from_static(concat!(
		"patr-cli/",
		env!("CARGO_PKG_VERSION_MAJOR"),
		".",
		env!("CARGO_PKG_VERSION_MINOR"),
		".",
		env!("CARGO_PKG_VERSION_PATCH"),
	));
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

/// Returns the path where a runner config file is stored for the given runner
/// type.
pub fn runner_config_path(runner_type: RunnerType) -> std::path::PathBuf {
	let name = match runner_type {
		RunnerType::Docker => "docker",
		RunnerType::Kubernetes => "kubernetes",
	};
	dirs::data_local_dir()
		.expect("Failed to get local data directory")
		.join("patr-cli")
		.join(format!("runner.{name}.json"))
}

/// Clears the terminal screen and moves the cursor to the top-left.
pub fn clear_screen() {
	let _ = crossterm::execute!(
		std::io::stderr(),
		crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
		crossterm::cursor::MoveTo(0, 0)
	);
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
