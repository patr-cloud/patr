use std::{
	fmt::{Display, Formatter},
	net::SocketAddr,
	str::FromStr,
};

use config::{Config, ConfigError, Environment, File};
use models::prelude::*;
use serde::{Deserialize, Serialize};

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSettings<D> {
	/// The mode the runner is running in. This will determine if the runner is
	/// running in self-hosted mode or managed mode (connecting to the Patr
	/// server).
	#[serde(flatten)]
	pub mode: RunnerMode,
	/// The environment the application is running in. This is set at runtime
	/// based on an environment variable and if the application is compiled with
	/// debug mode.
	pub environment: RunningEnvironment,
	/// The configuration for the database to connect to
	pub database: DatabaseConfig,
	/// The address to listen on
	#[serde(alias = "bindaddress")]
	pub bind_address: SocketAddr,
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

	/// Convert the the runner settings into a base runner setting, with the
	/// additional data as [`()`]. This allows the settings to be parsed and
	/// used internally in the common runner library without regard for the
	/// specific runner settings.
	#[instrument(skip(self))]
	pub fn into_base(self) -> RunnerSettings<()> {
		let RunnerSettings {
			mode,
			environment,
			database,
			bind_address,
			data: _,
		} = self;

		RunnerSettings::<()> {
			mode,
			environment,
			database,
			bind_address,
			data: (),
		}
	}
}

/// The mode the runner is running in. This will determine if the runner is
/// running in self-hosted mode or managed mode (connecting to the Patr server).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "mode")]
pub enum RunnerMode {
	/// This runner is running in self-hosted mode. This means that the runner
	/// will run all deployments on the runner itself.
	#[serde(rename_all = "camelCase")]
	SelfHosted {
		/// The Pepper used to hash passwords
		#[serde(alias = "passwordpepper")]
		password_pepper: String,
		/// The secret used to sign JWTs
		#[serde(alias = "jwtsecret")]
		jwt_secret: String,
	},
	/// This runner is running in managed mode. This means that the runner will
	/// connect to the Patr server to get the deployments to run.
	#[serde(rename_all = "camelCase")]
	Managed {
		/// The workspace ID to connect the runner for.
		#[serde(alias = "workspaceid")]
		workspace_id: Uuid,
		/// The runner ID to connect the runner for.
		#[serde(alias = "runnerid")]
		runner_id: Uuid,
		/// The bearer token for the runner to access the API
		#[serde(alias = "apitoken")]
		api_token: BearerToken,
		/// The user agent that the runner uses to access the API
		#[serde(skip, default = "get_user_agent")]
		user_agent: UserAgent,
	},
}

impl RunnerMode {
	/// Check if the runner is running in self-hosted mode
	pub fn is_self_hosted(&self) -> bool {
		matches!(self, RunnerMode::SelfHosted { .. })
	}

	/// Check if the runner is running in managed mode
	pub fn is_managed(&self) -> bool {
		matches!(self, RunnerMode::Managed { .. })
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
	#[serde(alias = "connectionlimit")]
	pub connection_limit: u32,
}
