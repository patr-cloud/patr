use std::{
	env,
	fmt::{Display, Formatter},
	net::SocketAddr,
};

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

use crate::prelude::*;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
	pub bind_addr: SocketAddr,
	pub api_base_path: String,
	pub password_pepper: String,
	pub jwt_secret: String,
	pub environment: RunningEnvironment,
	pub s3: S3Config,
	pub database: DatabaseConfig,
	pub redis: RedisConfig,
	// pub email: EmailConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunningEnvironment {
	Development,
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
