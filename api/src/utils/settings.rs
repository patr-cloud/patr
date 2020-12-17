use std::{
	env,
	fmt::{Display, Formatter},
};

use config_rs::{Config, Environment, File};
use serde_derive::Deserialize;

pub fn parse_config() -> Settings {
	println!("[TRACE]: Reading config data...");
	let mut settings = Config::new();
	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		env::var("APP_ENV").unwrap_or_else(|_| "dev".into())
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
				.merge(File::with_name("config/dev.sample.json"))
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
		.merge(Environment::with_prefix("APP_"))
		.expect("unable to merge with environment variables");

	settings.try_into().expect("unable to parse settings")
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub port: u16,
	pub base_path: String,
	pub password_salt: String,
	pub jwt_secret: String,
	pub environment: RunningEnvironment,
	pub s3: S3Settings,
	pub mysql: MySQLSettings,
	pub mongodb: MongoDBSettings,
	pub redis: RedisSettings,
	pub email: EmailSettings,
	pub twilio: TwilioSettings,
	pub cloudflare: CloudflareSettings,
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
pub struct MySQLSettings {
	pub host: String,
	pub port: u16,
	pub user: String,
	pub password: String,
	pub database: String,
	pub connection_limit: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MongoDBSettings {
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

impl Display for RunningEnvironment {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		formatter.write_str(match self {
			RunningEnvironment::Development => "Development",
			RunningEnvironment::Production => "Production",
		})
	}
}
