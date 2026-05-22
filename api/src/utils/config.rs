use std::{
	env,
	fmt::{Display, Formatter},
	net::SocketAddr,
};

use config::{Case, Config, Environment, File};
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
	// Env-var keys use `__` as the nested-key separator and `_` as the word
	// boundary inside each segment; `convert_case(Camel)` then matches the
	// `#[serde(rename_all = "camelCase")]` field names. E.g.
	// PATR__DATABASE__CONNECTION_LIMIT → database.connectionLimit.
	.add_source(
		Environment::with_prefix("PATR")
			.separator("__")
			.convert_case(Case::Camel),
	)
	.build()
	.expect("unable to merge with environment variables")
	.try_deserialize()
	.expect("unable to parse settings")
}

/// The global application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
	/// HTTP server binding configuration
	pub server: ServerConfig,
	/// The pepper used to hash passwords
	pub password_pepper: String,
	/// The secret used to sign JWTs
	pub jwt_secret: String,
	/// This is the primary domain that all deployments and all user-facing URLs
	/// will be hosted on
	#[cfg(feature = "cloud")]
	pub primary_hosted_domain: String,
	/// The environment the application is running in. This is set at runtime
	/// based on an environment variable and if the application is compiled with
	/// debug mode.
	pub environment: RunningEnvironment,
	/// The configuration for sending emails via SMTP
	pub email: EmailConfig,
	/// The configuration for S3, used for storing layers of docker images
	pub s3: S3Config,
	/// The configuration for the database to connect to
	pub database: DatabaseConfig,
	/// The configuration for Redis. This is used for caching, rate limiting and
	/// for subscribing to events from the database on websockets
	pub redis: RedisConfig,
	// pub email: EmailConfig,
	/// The cloudflare settings to use for the API
	#[cfg(feature = "cloud")]
	pub cloudflare: CloudflareConfig,
	/// The opentelemetry endpoint to send traces to
	pub opentelemetry: OpenTelemetryConfig,
	/// The configuration for IpInfo to get IpAddress details
	#[cfg(feature = "cloud")]
	pub ipinfo: IpInfoConfig,
	/// The configuration for social login providers (GitHub, etc.)
	#[cfg(feature = "cloud")]
	pub social_login: SocialLoginConfig,
	/// Knobs for the OCI registry endpoints
	pub registry: RegistryConfig,
}

/// OCI registry settings — currently the values surfaced in the
/// `WWW-Authenticate` Bearer challenge that docker clients use to scope
/// credentials and locate the token endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryConfig {
	/// The `service="..."` value in the Bearer challenge. Docker scopes
	/// credentials in `~/.docker/config.json` by this string.
	pub service: String,
	/// The `realm="..."` URL in the Bearer challenge. Must be reachable
	/// from the docker client (not just the API host).
	pub realm: String,
}

/// HTTP server binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
	/// The address to listen on
	pub bind_address: SocketAddr,
	/// The base path of the API
	pub api_base_path: String,
	/// The canonical base domain the API is served on. In cloud, this is the
	/// root of the platform (e.g. `patr.cloud`) and sub-services live on
	/// `api.`, `app.`, `registry.` subdomains. In self-hosted, this is the
	/// single domain that path-routes everything.
	pub base_domain: String,
}

/// The configuration for social login providers
#[cfg(feature = "cloud")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialLoginConfig {
	/// The configuration for GitHub OAuth2 SSO
	pub github: GitHubOAuthConfig,
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
#[cfg(feature = "cloud")]
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
	/// The base URL for the Cloudflare API. Defaults to
	/// `https://api.cloudflare.com/client/v4` in production.
	/// Override in tests to point at a mock server.
	#[serde(default = "default_cloudflare_base_url")]
	pub base_url: String,
}

/// The default base URL for the Cloudflare API
#[cfg(feature = "cloud")]
fn default_cloudflare_base_url() -> String {
	"https://api.cloudflare.com/client/v4/".to_string()
}

/// The configuration for sending emails via SMTP
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailConfig {
	/// The SMTP server hostname
	pub host: String,
	/// The SMTP server port
	pub port: u16,
	/// Whether to use TLS for the SMTP connection
	pub secure: bool,
	/// The SMTP username for authentication
	pub username: String,
	/// The email address to use as the sender for all emails
	pub from: String,
	/// The SMTP password for authentication
	pub password: String,
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
#[cfg(feature = "cloud")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpInfoConfig {
	/// The token for connecting to ipinfo.io
	pub token: String,
}

/// The configuration for GitHub OAuth2, used to allow users to sign in with
/// their GitHub account.
///
/// The GitHub OAuth App's registered Authorization callback URL should be
/// the site root (`https://app.patr.cloud/`) — the only common parent of
/// the two callback URLs below. GitHub allows any `redirect_uri` that is a
/// subpath of the registered URL.
#[cfg(feature = "cloud")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubOAuthConfig {
	/// The Client ID of the GitHub OAuth App registered at
	/// https://github.com/settings/developers
	pub client_id: String,
	/// The Client Secret of the GitHub OAuth App
	pub client_secret: String,
	/// Frontend page that GitHub redirects to after the unauthenticated
	/// sign-in flow. In production: `https://app.patr.cloud/login/github`.
	pub callback_url: String,
	/// Frontend page that GitHub redirects to after the authenticated
	/// "Connect GitHub" flow from Profile → Connected Accounts.
	/// In production: `https://app.patr.cloud/profile/github/callback`.
	pub connect_callback_url: String,
}
