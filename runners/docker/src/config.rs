use std::fmt::{Display, Formatter};

use config::{Config, Environment, File};
use models::prelude::*;
use serde::{Deserialize, Serialize};

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSettings {
	/// The workspace ID to connect the runner for.
	#[serde(rename = "workspaceid")]
	pub workspace_id: Uuid,
	/// The runner ID to connect the runner for.
	#[serde(rename = "runnerid")]
	pub runner_id: Uuid,
	/// The API token to authenticate the runner with.
	#[serde(rename = "apitoken")]
	pub api_token: String,
	/// The environment to run the runner in.
	pub environment: RunningEnvironment,
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

/// Get the runner settings from the environment.
pub fn get_runner_settings() -> RunnerSettings {
	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		std::env::var("PATR_ENV").unwrap_or_else(|_| "prod".into())
	};

	match env.as_ref() {
		"prod" | "production" => Config::builder()
			.add_source(File::with_name("config").required(false))
			.set_default("environment", "production")
			.expect("unable to set environment to production"),
		"dev" | "development" => Config::builder()
			.add_source(
				File::with_name(concat!(
					env!("CARGO_MANIFEST_DIR"),
					"/../../config/runner.docker"
				))
				.required(true),
			)
			.set_default("environment", "development")
			.expect("unable to set environment to development"),
		_ => {
			panic!("Unknown running environment found!");
		}
	}
	.add_source(Environment::with_prefix("PATR").separator("_"))
	.build()
	.expect("unable to merge with environment variables")
	.try_deserialize()
	.expect("unable to parse settings")
}
