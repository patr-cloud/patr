use std::fmt::{Display, Formatter};

use config::{Config, ConfigError, Environment, File};
use models::prelude::*;
use serde::{Deserialize, Serialize};

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSettings<D> {
	/// The workspace ID to connect the runner for.
	#[serde(rename = "workspaceid")]
	pub workspace_id: Uuid,
	/// The runner ID to connect the runner for.
	#[serde(rename = "runnerid")]
	pub runner_id: Uuid,
	/// The API token to authenticate the runner with.
	#[serde(rename = "apitoken")]
	pub api_token: String,
	/// The environment the application is running in. This is set at runtime
	/// based on an environment variable and if the application is compiled with
	/// debug mode.
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
	pub fn parse(name: &str) -> Result<Self, ConfigError> {
		let env = if cfg!(debug_assertions) {
			"dev".to_string()
		} else {
			std::env::var("PATR_ENV").unwrap_or_else(|_| "prod".into())
		};

		match env.as_ref() {
			"prod" | "production" => Config::builder()
				.add_source(File::with_name("config").required(false))
				.add_source(File::with_name(&format!("config.{}", name)).required(false))
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
		.add_source(Environment::with_prefix("PATR").separator("_"))
		.build()?
		.try_deserialize()
	}
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
	#[serde(alias = "connectionlimit")]
	pub connection_limit: u32,
}
