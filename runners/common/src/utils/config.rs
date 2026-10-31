use std::{
	fmt::{Display, Formatter},
	str::FromStr,
};

use config::{Case, Config, ConfigError, Environment, File};
use models::prelude::*;
use serde::{Deserialize, Serialize};

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSettings<D> {
	/// The workspace ID to connect the runner for.
	pub workspace_id: Uuid,
	/// The runner ID to connect the runner for.
	pub runner_id: Uuid,
	/// The bearer token for the runner to access the API
	pub api_token: BearerToken,
	/// The user agent that the runner uses to access the API
	#[serde(skip, default = "get_user_agent")]
	pub user_agent: UserAgent,
	/// The environment the application is running in. This is set at runtime
	/// based on an environment variable and if the application is compiled with
	/// debug mode.
	#[serde(skip, default = "default_running_environment")]
	pub environment: RunningEnvironment,
	/// The configuration for the database to connect to
	pub database: DatabaseConfig,
	/// Additional settings for the runner.
	#[serde(flatten)]
	pub data: D,
}

impl<'de, D> RunnerSettings<D>
where
	D: Deserialize<'de> + Serialize,
{
	/// Get the runner settings from the environment.
	#[instrument]
	pub fn parse(name: &str) -> Result<Self, ConfigError> {
		let env = if cfg!(debug_assertions) {
			"dev".to_string()
		} else {
			std::env::var("PATR_ENV").unwrap_or_else(|_| "prod".into())
		};

		match env.as_ref() {
			"prod" | "production" => Config::builder()
				.add_source(File::with_name("config").required(false))
				.add_source(File::with_name(&format!("config.{name}")).required(false))
				.add_source(File::with_name(name).required(false))
				.set_default("environment", "production")?,
			"dev" | "development" => Config::builder()
				.add_source(
					File::with_name(&format!(
						concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/runner.{}",),
						name
					))
					.required(true),
				)
				.set_default("environment", "development")?,
			_ => {
				panic!("Unknown running environment found!");
			}
		}
		// Env-var keys use `__` as the nested-key separator and `_` as the word
		// boundary inside each segment; `convert_case(Camel)` then matches the
		// `#[serde(rename_all = "camelCase")]` field names. E.g.
		// PATR__DATABASE__CONNECTION_LIMIT → database.connectionLimit.
		.add_source(
			Environment::with_prefix("PATR")
				.separator("__")
				.convert_case(Case::Camel)
				// Coerce env-var strings to bool/number so typed fields (e.g.
				// the u32 `connection_limit`, the u16 `ingress_http_listen_port`)
				// deserialize from environment overrides. Without this, a value
				// like "5" fails with "invalid type: string, expected u32".
				.try_parsing(true),
		)
		.build()?
		.try_deserialize()
	}

	/// Convert the the runner settings into a base runner setting, with the
	/// additional data as [`()`]. This allows the settings to be parsed and
	/// used internally in the common runner library without regard for the
	/// specific runner settings.
	#[instrument(skip(self))]
	pub fn into_base(self) -> RunnerSettings<()> {
		let RunnerSettings {
			workspace_id,
			runner_id,
			api_token,
			user_agent,
			environment,
			database,
			data: _,
		} = self;

		RunnerSettings::<()> {
			workspace_id,
			runner_id,
			api_token,
			user_agent,
			environment,
			database,
			data: (),
		}
	}
}

/// Get the default running environment based on whether the application is
/// compiled in debug mode or not, and the `PATR_ENV` environment variable. If
/// the application is compiled in debug mode, the default environment is
/// development. Otherwise, it checks the `PATR_ENV` environment variable and
/// defaults to production if it is not set or if it is set to an unknown value.
/// The environment variable can be set to either "dev" or "development" for
/// development mode, and "prod" or "production" for production mode.
fn default_running_environment() -> RunningEnvironment {
	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		std::env::var("PATR_ENV").unwrap_or_else(|_| "prod".into())
	};

	match env.as_ref() {
		"dev" | "development" => RunningEnvironment::Development,
		"prod" | "production" => RunningEnvironment::Production,
		_ => panic!("Unknown running environment found!"),
	}
}

/// Get the user agent for the runner
fn get_user_agent() -> UserAgent {
	UserAgent::from_str(concat!(
		env!("CARGO_PKG_NAME"),
		"/",
		env!("CARGO_PKG_VERSION"),
	))
	.expect("Failed to parse user agent as valid header")
}

/// The environment the application is running in
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RunningEnvironment {
	/// The application is running in development mode
	Development,
	/// The application is running in production mode
	Production,
}

impl Display for RunningEnvironment {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			formatter,
			"{}",
			match self {
				RunningEnvironment::Development => "Development",
				RunningEnvironment::Production => "Production",
			}
		)
	}
}

/// The configuration for the database to connect to. This will be the primary
/// data store for all information contained in the API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfig {
	/// The location of the sqlite database file
	pub file: String,
	/// The maximum number of connections to the database
	pub connection_limit: u32,
}
