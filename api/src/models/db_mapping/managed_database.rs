use std::{fmt::Display, str::FromStr};

use eve_rs::AsError;

use crate::{error, utils::Error};

// TODO: implement other queries for this
pub struct ManagedDatabase {
	pub id: Vec<u8>,
	pub name: String,
	pub status: ManagedDatabaseStatus,
	pub database_id: Option<String>,
	pub db_service: Option<String>,
	pub organisation_id: Vec<u8>,
}

#[derive(sqlx::Type, Debug)]
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
			"creating" => Ok(Self::Creating),
			"running" => Ok(Self::Running),
			"errored" => Ok(Self::Errored),
			"deleted" => Ok(Self::Deleted),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}
