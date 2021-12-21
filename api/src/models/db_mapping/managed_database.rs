use std::{fmt::Display, str::FromStr};

use api_models::utils::Uuid;
use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use crate::{error, utils::Error};

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug)]
#[sqlx(type_name = "MANAGED_DATABASE_PLAN", rename_all = "lowercase")]
pub enum ManagedDatabasePlan {
	Nano,
	Micro,
	Small,
	Medium,
	Large,
	Xlarge,
	Xxlarge,
	Mammoth,
}

impl ManagedDatabasePlan {
	pub fn as_do_plan(&self) -> Result<String, Error> {
		match self {
			Self::Nano => Ok("db-s-1vcpu-1gb"),
			Self::Micro => Ok("db-s-1vcpu-2gb"),
			Self::Medium => Ok("db-s-2vcpu-4gb"),
			Self::Large => Ok("db-s-4vcpu-8gb"),
			Self::Xlarge => Ok("db-s-6vcpu-16gb"),
			Self::Xxlarge => Ok("db-s-8vcpu-32gb"),
			Self::Mammoth => Ok("db-s-16vcpu-64gb"),
			_ => Err(Error::empty()),
		}
		.map(|value| value.to_string())
	}

	pub fn as_aws_plan(&self) -> Result<String, Error> {
		match self {
			Self::Micro => Ok("micro_1_0"),
			Self::Small => Ok("small_1_0"),
			Self::Medium => Ok("medium_1_0"),
			Self::Large => Ok("large_1_0"),
			_ => Err(Error::empty()),
		}
		.map(|value| value.to_string())
	}
}

impl Display for ManagedDatabasePlan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Nano => write!(f, "nano"),
			Self::Micro => write!(f, "micro"),
			Self::Small => write!(f, "small"),
			Self::Medium => write!(f, "medium"),
			Self::Large => write!(f, "large"),
			Self::Xlarge => write!(f, "xlarge"),
			Self::Xxlarge => write!(f, "xxlarge"),
			Self::Mammoth => write!(f, "mammoth"),
		}
	}
}

impl FromStr for ManagedDatabasePlan {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"do-nano" | "db-s-1vcpu-1gb" => Ok(Self::Nano),
			"do-micro" | "db-s-1vcpu-2gb" | "aws-micro" | "micro_1_0" => {
				Ok(Self::Micro)
			}
			"aws-small" | "small_1_0" => Ok(Self::Small),
			"do-medium" | "db-s-2vcpu-4gb" | "aws-medium" | "medium_1_0" => {
				Ok(Self::Medium)
			}
			"do-large" | "db-s-4vcpu-8gb" | "aws-large" | "large_1_0" => {
				Ok(Self::Large)
			}
			"do-xlarge" | "db-s-6vcpu-16gb" => Ok(Self::Xlarge),
			"do-xxlarge" | "db-s-8vcpu-32gb" => Ok(Self::Xxlarge),
			"do-mammoth" | "db-s-16vcpu-64gb" => Ok(Self::Mammoth),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ManagedDatabase {
	pub id: Uuid,
	pub name: String,
	pub db_name: String,
	pub engine: ManagedDatabaseEngine,
	pub version: String,
	pub num_nodes: i32,
	pub database_plan: ManagedDatabasePlan,
	pub region: String,
	pub status: ManagedDatabaseStatus,
	pub host: String,
	pub port: i32,
	pub username: String,
	pub password: String,
	pub workspace_id: Uuid,
	pub digitalocean_db_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug)]
#[sqlx(type_name = "MANAGED_DATABASE_STATUS", rename_all = "lowercase")]
pub enum ManagedDatabaseStatus {
	Creating,
	Running,
	Errored,
	Deleted,
}

impl Display for ManagedDatabaseStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Creating => write!(f, "creating"),
			Self::Running => write!(f, "running"),
			Self::Errored => write!(f, "errored"),
			Self::Deleted => write!(f, "deleted"),
		}
	}
}

impl FromStr for ManagedDatabaseStatus {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"configuring-log-exports" |
			"started" |
			"creating" |
			"backing-up" |
			"notstarted" => Ok(Self::Creating),
			"running" | "online" | "created" | "completed" | "succeeded" |
			"available" => Ok(Self::Running),
			"errored" | "failed" => Ok(Self::Errored),
			"deleted" => Ok(Self::Deleted),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "MANAGED_DATABASE_ENGINE", rename_all = "lowercase")]
pub enum ManagedDatabaseEngine {
	Postgres,
	Mysql,
}

impl Display for ManagedDatabaseEngine {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Postgres => write!(f, "postgres"),
			Self::Mysql => write!(f, "mysql"),
		}
	}
}

impl FromStr for ManagedDatabaseEngine {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"pg" | "postgres" | "postgresql" => Ok(Self::Postgres),
			"mysql" => Ok(Self::Mysql),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}
