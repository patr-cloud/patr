use std::{fmt::Display, str::FromStr};

use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use super::CloudPlatform;
use crate::{error, utils::Error};

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug)]
#[sqlx(type_name = "DATABASE_PLAN", rename_all = "lowercase")]
pub enum DatabasePlan {
	DoNano,
	DoMicro,
	DoMedium,
	DoLarge,
	DoXlarge,
	DoXxlarge,
	DoMammoth,
	AwsMicro,
	AwsSmall,
	AwsMedium,
	AwsLarge,
}

impl Display for DatabasePlan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			DatabasePlan::DoNano => write!(f, "db-s-1vcpu-1gb"),
			DatabasePlan::DoMicro => write!(f, "db-s-1vcpu-2gb"),
			DatabasePlan::DoMedium => write!(f, "db-s-2vcpu-4gb"),
			DatabasePlan::DoLarge => write!(f, "db-s-4vcpu-8gb"),
			DatabasePlan::DoXlarge => write!(f, "db-s-6vcpu-16gb"),
			DatabasePlan::DoXxlarge => write!(f, "db-s-8vcpu-32gb"),
			DatabasePlan::DoMammoth => write!(f, "db-s-16vcpu-64gb"),
			DatabasePlan::AwsMicro => write!(f, "micro_1_0"),
			DatabasePlan::AwsSmall => write!(f, "small_1_0"),
			DatabasePlan::AwsMedium => write!(f, "medium_1_0"),
			DatabasePlan::AwsLarge => write!(f, "large_1_0"),
		}
	}
}

impl FromStr for DatabasePlan {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"do-nano" | "db-s-1vcpu-1gb" => Ok(DatabasePlan::DoNano),
			"do-micro" | "db-s-1vcpu-2gb" => Ok(DatabasePlan::DoMicro),
			"do-medium" | "db-s-2vcpu-4gb" => Ok(DatabasePlan::DoMedium),
			"do-large" | "db-s-4vcpu-8gb" => Ok(DatabasePlan::DoLarge),
			"do-xlarge" | "db-s-6vcpu-16gb" => Ok(DatabasePlan::DoXlarge),
			"do-xxlarge" | "db-s-8vcpu-32gb" => Ok(DatabasePlan::DoXxlarge),
			"do-mammoth" | "db-s-16vcpu-64gb" => Ok(DatabasePlan::DoMammoth),
			"aws-micro" | "micro_1_0" => Ok(DatabasePlan::AwsMicro),
			"aws-small" | "small_1_0" => Ok(DatabasePlan::AwsSmall),
			"aws-medium" | "medium_1_0" => Ok(DatabasePlan::AwsMedium),
			"aws-large" | "large_1_0" => Ok(DatabasePlan::AwsLarge),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ManagedDatabase {
	pub id: Vec<u8>,
	pub name: String,
	pub db_name: String,
	pub engine: Engine,
	pub version: String,
	pub num_nodes: i32,
	pub size: String,
	pub region: String,
	pub status: ManagedDatabaseStatus,
	pub host: String,
	pub port: i32,
	pub username: String,
	pub password: String,
	pub organisation_id: u8,
	pub digital_ocean_db_id: Option<String>,
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
			"creating" |
			"configuring-log-exports" |
			"backing-up" |
			"Started" |
			"NotStarted" => Ok(Self::Creating),
			"running" | "online" | "created" | "Completed" | "Succeeded" |
			"available" => Ok(Self::Running),
			"errored" | "failed" | "Failed" => Ok(Self::Errored),
			"deleted" => Ok(Self::Deleted),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "ENGINE", rename_all = "lowercase")]
pub enum Engine {
	Postgres,
	Mysql,
}

impl Display for Engine {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Postgres => write!(f, "postgres"),
			Self::Mysql => write!(f, "mysql"),
		}
	}
}

impl FromStr for Engine {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"pg" | "postgres" | "postgresql" | "Postgres" | "Postgresql" |
			"POSTGRESQL" | "POSTGRES" => Ok(Self::Postgres),
			"mysql" | "Mysql" | "MYSQL" => Ok(Self::Mysql),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}
