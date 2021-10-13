use std::{
	env,
	fmt::{Display, Formatter},
};

use config_rs::{Config, Environment, File};
use serde::Deserialize;

pub fn parse_config() -> Settings {
	println!("[TRACE]: Reading config data...");
	let mut settings = Config::new();
	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		env::var("APP_ENV").unwrap_or_else(|_| "prod".into())
	};

	match env.as_ref() {
		"prod" | "production" => {
			settings
				.merge(File::with_name("config/prod"))
				.expect("unable to find prod config");
			settings
				.set("environment", "production")
				.expect("unable to set running environment");
		}
		"dev" | "development" => {
			settings
				.merge(File::with_name("config/dev"))
				.expect("unable to find dev config");
			settings
				.set("environment", "development")
				.expect("unable to set running environment");
		}
		_ => {
			panic!("Unknown running environment found!");
		}
	}

	settings
		.merge(Environment::with_prefix("APP").separator("_"))
		.expect("unable to merge with environment variables");

	let mut settings: Settings =
		settings.try_into().expect("unable to parse settings");

	settings.docker_registry.public_key_der =
		Some(base64::decode(&settings.docker_registry.public_key).unwrap());

	settings
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub port: u16,
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
	pub digital_ocean_api_key: String,
	pub ssh: SshSettings,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct S3Settings {
	pub endpoint: String,
	pub region: String,
	pub bucket: String,
	pub key: String,
	pub secret: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSettings {
	pub host: String,
	pub port: u16,
	pub user: String,
	pub password: String,
	pub database: String,
	pub connection_limit: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MongoDbSettings {
	pub host: String,
	pub port: u16,
	pub user: Option<String>,
	pub password: Option<String>,
	pub database: String,
	pub connection_limit: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RedisSettings {
	pub host: String,
	pub port: u16,
	pub user: Option<String>,
	pub password: Option<String>,
	pub database: Option<String>,
	pub connection_limit: u32,
	pub secure: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TwilioSettings {
	pub username: String,
	pub access_token: String,
	pub from_number: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmailSettings {
	pub host: String,
	pub port: u16,
	pub secure: bool,
	pub username: String,
	pub from: String,
	pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareSettings {
	pub account_id: String,
	pub account_email: String,
	pub api_token: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(into = "String", rename_all = "camelCase")]
pub enum RunningEnvironment {
	Development,
	Production,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerRegistrySettings {
	pub service_name: String,
	pub issuer: String,
	pub registry_url: String,
	pub private_key: String,
	pub public_key: String,
	pub public_key_der: Option<Vec<u8>>,
}

impl DockerRegistrySettings {
	pub fn public_key_der(&self) -> &[u8] {
		self.public_key_der.as_ref().unwrap().as_ref()
	}
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

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SshSettings {
	pub host: String,
	pub port: u16,
	pub username: String,
	pub key_file: String,
}
