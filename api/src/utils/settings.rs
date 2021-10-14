use std::{
	env,
	fmt::{Display, Formatter},
	net::IpAddr,
};

use config_rs::{Config, Environment, File};
use serde::{Deserialize, Deserializer, Serialize};

pub fn parse_config() -> Settings {
	println!("[TRACE]: Reading config data...");
	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		env::var("APP_ENV").unwrap_or_else(|_| "prod".into())
	};

	match env.as_ref() {
		"prod" | "production" => Config::builder()
			.add_source(File::with_name("config/prod").required(false))
			.set_default("environment", "production")
			.expect("unable to set environment to develop"),
		"dev" | "development" => Config::builder()
			.add_source(File::with_name("config/dev").required(false))
			.set_default("environment", "development")
			.expect("unable to set environment to develop"),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub port: u16,
	pub bind_address: IpAddr,
	pub base_path: String,
	pub password_pepper: String,
	pub jwt_secret: String,
	pub environment: RunningEnvironment,
	pub s3: S3Settings,
	pub database: DatabaseSettings,
	pub mongodb: MongoDbSettings,
	pub redis: RedisSettings,
	pub email: EmailSettings,
	pub twilio: TwilioSettings,
	pub cloudflare: CloudflareSettings,
	pub docker_registry: DockerRegistrySettings,
	pub digitalocean: Digitalocean,
	pub kubernetes: KubernetesSettings,
	pub prometheus: PrometheusSettings,
	pub chargebee: ChargebeeSettings,
	pub rabbit_mq: RabbitMqSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Settings {
	pub endpoint: String,
	pub region: String,
	pub bucket: String,
	pub key: String,
	pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSettings {
	pub host: String,
	pub port: u16,
	pub user: String,
	pub password: String,
	pub database: String,
	pub connection_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoDbSettings {
	pub host: String,
	pub port: u16,
	pub user: Option<String>,
	pub password: Option<String>,
	pub database: String,
	pub connection_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisSettings {
	pub host: String,
	pub port: u16,
	pub user: Option<String>,
	pub password: Option<String>,
	pub database: Option<u8>,
	pub connection_limit: u32,
	pub secure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwilioSettings {
	pub username: String,
	pub access_token: String,
	pub from_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailSettings {
	pub host: String,
	pub port: u16,
	pub secure: bool,
	pub username: String,
	pub from: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareSettings {
	pub account_id: String,
	pub account_email: String,
	pub api_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunningEnvironment {
	Development,
	Production,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerRegistrySettings {
	pub service_name: String,
	pub issuer: String,
	pub registry_url: String,
	pub private_key: String,
	pub public_key: String,
	#[serde(deserialize_with = "base64_to_byte_array")]
	pub public_key_der: Vec<u8>,
	pub authorization_header: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Digitalocean {
	pub api_key: String,
	pub registry: String,
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
pub struct KubernetesSettings {
	pub certificate_authority_data: String,
	pub cluster_name: String,
	pub cluster_url: String,
	pub auth_name: String,
	pub auth_username: String,
	pub auth_token: String,
	pub context_name: String,
	pub cert_issuer_http: String,
	pub cert_issuer_dns: String,
	pub static_site_proxy_service: String,
}

fn base64_to_byte_array<'de, D>(value: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let string = String::deserialize(value)?;
	Ok(base64::decode(&string)
		.unwrap_or_else(|_| panic!("Unable to decode {} as base64", string)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusSettings {
	pub host: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargebeeSettings {
	pub api_key: String,
	pub url: String,
	pub credit_amount: String,
	pub description: String,
	pub gateway_id: String,
	pub redirect_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RabbitMqSettings {
	pub host: String,
	pub port: u16,
	pub queue: String,
}
