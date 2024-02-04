use std::{
	env,
	fmt::{Display, Formatter},
	net::SocketAddr,
};

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

/// Parses the configuration of the application and returns the parsed config.
/// In case of any errors while parsing, this function will panic.
///
/// This should ideally be only called once during initialization and the parsed
/// config should be used for the lifetime of the application.
pub fn parse_config() -> AppConfig {
	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		env::var("PATR_ENV").unwrap_or_else(|_| "prod".into())
	};

	match env.as_ref() {
		"prod" | "production" => Config::builder()
			.add_source(File::with_name("config").required(false))
			.set_default("environment", "production")
			.expect("unable to set environment to production"),
		"dev" | "development" => Config::builder()
			.add_source(
				File::with_name(concat!(env!("CARGO_MANIFEST_DIR"), "/../config/dev"))
					.required(false),
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
	/// The opentelemetry endpoint to send traces to
	pub opentelemetry: OpenTelemetryConfig,
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

/// The configuration for S3, where objects and large files used by the API will
/// be stored in
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
	/// The endpoint of the S3 server
	pub endpoint: String,
	/// The region of the S3 server
	pub region: String,
	/// The bucket to store objects in
	pub bucket: String,
	/// The access key to use to connect to the S3 server
	pub key: String,
	/// The secret key to use to connect to the S3 server
	pub secret: String,
}

/// The configuration for the database to connect to. This will be the primary
/// data store for all information contained in the API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfig {
	/// The host of the database
	pub host: String,
	/// The port of the database
	pub port: u16,
	/// The username to use to connect to the database
	pub user: String,
	/// The password to use to connect to the database
	pub password: String,
	/// The name of the database to connect to within the database server
	pub database: String,
	/// The maximum number of connections to the database
	pub connection_limit: serde_json::value::Number,
}

/// The configuration for Redis. This is used for caching, rate limiting and for
/// subscribing to events from the database on websockets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisConfig {
	/// The host of the Redis server
	pub host: String,
	/// The port of the Redis server
	pub port: u16,
	/// The username to use to connect to the Redis server, if any
	pub user: Option<String>,
	/// The password to use to connect to the Redis server, if any
	pub password: Option<String>,
	/// The database to use within the Redis server. The default is 0
	#[serde(default = "default_redis_database")]
	pub database: u8,
	/// Whether or not to use TLS to connect to the Redis server
	pub secure: bool,
}

fn default_redis_database() -> u8 {
	0
}

/// The configuration for the SMTP server to use to send emails to users
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailConfig {
	/// The host of the SMTP server
	pub host: String,
	/// The port of the SMTP server
	pub port: u16,
	/// Whether or not to use TLS to connect to the SMTP server
	pub secure: bool,
	/// The username to use to connect to the SMTP server
	pub username: String,
	/// The from address to use when sending emails
	pub from: String,
	/// The password to use to connect to the SMTP server
	pub password: String,
}

/// The configuration for the opentelemetry endpoint to send traces to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTelemetryConfig {
	/// The endpoint to send traces to
	pub endpoint: String,
}
