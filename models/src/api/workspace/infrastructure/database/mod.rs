use serde::{Deserialize, Serialize};

use crate::prelude::Uuid;

mod create_database;
mod delete_managed_database;
mod get_managed_database;
mod list_managed_database;

pub use self::{
	create_database::*,
	delete_managed_database::*,
	get_managed_database::*,
	list_managed_database::*,
};

/// Information to connect to the database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConnection {
	pub host: String,
	pub port: u32,
	pub username: String,
	pub password: String,
}

/// Supported databases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseEngine {
	Postgres,
	Mysql,
	Mongo,
	Redis,
}

/// Possible database status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseStatus {
	Creating,
	Running,
	Errored,
	Deleted,
}

/// Database information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Database {
	pub id: Uuid,
	pub name: String,
	pub engine: DatabaseEngine,
	pub version: String,
	pub num_nodes: u16,
	pub database_plan_id: Uuid,
	pub region: Uuid,
	pub status: DatabaseStatus,
	pub public_connection: DatabaseConnection,
}
