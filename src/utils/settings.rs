use config_rs::{Config, Environment, File};
use std::env;

pub fn parse_config() -> Settings {
	println!("[TRACE]: Reading config data...");
	let mut settings = Config::new();

	match env::var("APP_ENV").unwrap_or_else(|_| "dev".into()).as_ref() {
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
		.merge(Environment::with_prefix("app"))
		.expect("unable to merge with environment variables");

	settings.try_into().expect("unable to parse settings")
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub port: u16,
	pub base_path: String,
	pub password_salt_rounds: u16,
	pub jwt_secret: String,
	pub environment: RunningEnvironment,
	pub s3: S3Settings,
	pub mysql: MySQLSettings,
	pub mongodb: MongoDBSettings,
	pub redis: RedisSettings,
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
#[serde(into = "String", rename_all = "camelCase")]
pub enum RunningEnvironment {
	Development,
	Production,
}

impl std::fmt::Display for RunningEnvironment {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str(match self {
			RunningEnvironment::Development => "Development",
			RunningEnvironment::Production => "Production",
		})
	}
}
