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
			.add_source(File::with_name("./config/api").required(false))
			.add_source(File::with_name("../config/api").required(false))
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
	/// This is the primary domain that all deployments and all user-facing URLs
	/// will be hosted on
	pub primary_hosted_domain: String,
	/// The environment the application is running in. This is set at runtime
	/// based on an environment variable and if the application is compiled with
	/// debug mode.
	pub environment: RunningEnvironment,
	/// The configuration for SendGrid to use to send emails to users such as
	/// signing up and for password resets
	pub send_grid: SendGridConfig,
	/// The configuration for S3, used for storing layers of docker images
	pub s3: S3Config,
	/// The configuration for the database to connect to
	pub database: DatabaseConfig,
	/// The configuration for Redis. This is used for caching, rate limiting and
	/// for subscribing to events from the database on websockets
	pub redis: RedisConfig,
	// pub email: EmailConfig,
	/// The cloudflare settings to use for the API
	pub cloudflare: CloudflareConfig,
	/// The opentelemetry endpoint to send traces to
	pub opentelemetry: OpenTelemetryConfig,
	/// The configuration for IpInfo to get IpAddress details
	pub ipinfo: IpInfoConfig,
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
	/// Whether to use path-style addressing for S3 requests.
	/// Required for MinIO and other S3-compatible stores that don't support
	/// virtual-hosted-style addressing.
	#[serde(default)]
	pub force_path_style: bool,
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
	pub connection_limit: u32,
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

/// The default value for the Redis database
fn default_redis_database() -> u8 {
	0
}

/// The configuration for Cloudflare to use for the API. This is used to
/// setup DNS records and for Cloudflare Tunnels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareConfig {
	/// The API key to use to connect to Cloudflare
	pub api_key: String,
	/// The account ID to use to connect to Cloudflare
	pub account_id: String,
	/// The namespace ID to use for Workers KV
	pub worker_namespace_id: String,
	/// The secret key to use for Cloudflare Turnstile
	pub turnstile_secret: String,
	/// The zone ID of the primary hosted zone
	pub primary_hosted_zone_id: String,
}

/// The configuration for SendGrid to use to send emails to users
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendGridConfig {
	/// The API key to use to connect to SendGrid
	pub api_key: String,
	/// The email address to use as the sender for all emails sent by the API
	pub from: String,
	/// The list of all templates to use for sending emails, mapped by a unique
	/// name for each template
	pub templates: SendGridTemplateConfig,
}

/// The list of all SendGrid templates to use for sending emails, mapped by a
/// unique name for each template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendGridTemplateConfig {
	pub user_sign_up: String,
	pub sign_up_completed: String,
}

/// The configuration for the opentelemetry endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTelemetryConfig {
	/// The metrics configuration for the opentelemetry endpoint
	pub tracing: TracingConfig,
	/// The loki configuration to use for logs
	pub logs: LogsConfig,
	/// The mimir configuration to use for metrics
	pub metrics: MetricsConfig,
}

/// The configuration for the opentelemetry endpoint to send traces to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracingConfig {
	/// The endpoint to send traces to
	pub endpoint: String,
}

/// The configuration for Loki to use for logs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsConfig {
	/// The endpoint to send logs to
	pub endpoint: String,
}

/// The configuration for Mimir to use for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsConfig {
	/// The endpoint to query for metrics
	pub endpoint: String,
	/// The username to use to connect to the Mimir server
	pub username: String,
	/// The password to use to connect to the Mimir server
	pub password: String,
}

/// The configuration for IpInfo to get information about an IP Address
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpInfoConfig {
	/// The token for connecting to ipinfo.io
	pub token: String,
}
