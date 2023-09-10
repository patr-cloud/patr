use std::{
	env,
	fmt::{Display, Formatter},
	net::SocketAddr,
};

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Parses the configuration of the application and returns the parsed config.
/// In case of any errors while parsing, this function will panic.
///
/// This should ideally be only called once during initialization and the parsed
/// config should be used for the lifetime of the application.
#[instrument]
pub fn parse_config() -> AppConfig {
	trace!("Reading config data...");

	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		env::var("APP_ENV").unwrap_or_else(|_| "prod".into())
	};

	match env.as_ref() {
		"prod" | "production" => Config::builder()
			.add_source(File::with_name("config/prod").required(false))
			.set_default("environment", "production")
			.expect("unable to set environment to production"),
		"dev" | "development" => Config::builder()
			.add_source(File::with_name("config/dev").required(false))
			.set_default("environment", "development")
			.expect("unable to set environment to development"),
		_ => {
			panic!("Unknown running environment found!");
		}
	}
	.add_source(Environment::with_prefix("APP").separator("_"))
	.build()
	.expect("unable to merge with environment variables")
	.try_deserialize()
	.expect("unable to parse settings")
}

/// The global application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
	/// The address to listed on
	pub bind_address: SocketAddr,
	/// The base path of the API
	pub api_base_path: String,
	/// The pepper used to hash passwords
	pub password_pepper: String,
	/// The secret used to sign JWTs
	pub jwt_secret: String,
	/// The environment the application is running in. This is set at runtime
	/// based on an environment variable and if the application is compiled with
	/// debug mode.
	pub environment: RunningEnvironment,
	/// The configuration for S3, used for storing layers of docker images
	pub s3: S3Config,
	/// The configuration for the database to connect to
	pub database: DatabaseConfig,
	/// The configuration for Redis. This is used for caching, rate limiting and
	/// for subscribing to events from the database on websockets
	pub redis: RedisConfig,
	// pub email: EmailConfig,
}

/// The environment the application is running in
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
	pub endpoint: String,
	pub region: String,
	pub bucket: String,
	pub key: String,
	pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfig {
	pub host: String,
	pub port: u16,
	pub user: String,
	pub password: String,
	pub database: String,
	pub connection_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisConfig {
	pub host: String,
	pub port: u16,
	pub user: Option<String>,
	pub password: Option<String>,
	#[serde(default = "default_redis_database")]
	pub database: u8,
	pub secure: bool,
}

fn default_redis_database() -> u8 {
	0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailConfig {
	pub host: String,
	pub port: u16,
	pub secure: bool,
	pub username: String,
	pub from: String,
	pub password: String,
}
