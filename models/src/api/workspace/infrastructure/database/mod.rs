use serde::{Deserialize, Serialize};

use crate::prelude::Uuid;

mod create_database;
mod delete_database;
mod get_database;
mod list_database;
mod list_all_database_machine_type;

pub use self::{
	create_database::*,
	delete_database::*,
	get_database::*,
	list_database::*,
	list_all_database_machine_type::*,
};

/// Information of all the different database plans currently supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabasePlan {
	/// The number of CPU nodes
	pub cpu_count: i32,
	/// The number of memory nodes
	pub memory_count: i32,
	/// The size of the volume
	pub volume: i32,
}
/// Information for the user to connect to the database instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConnection {
	/// The database host IP
	pub host: String,
	/// The connection port
	pub port: u32,
	/// The amin username
	pub username: String,
	/// The admin password
	pub password: String,
}

/// All the currrently supported databases offered to the users
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseEngine {
	/// version:
	Postgres,
	/// version:
	Mysql,
	/// version:
	Mongo,
	/// version:
	Redis,
}

/// All the possible status the database pod can be in during it's lifetime
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseStatus {
	/// Database is deploying
	Creating,
	/// Database is running and ready for connections
	Running,
	/// Database has stopped due to an error
	Errored,
	/// Database has being deleted and does not exist
	Deleted,
}

/// Database information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Database {
	/// Name of database entered by the user
	pub name: String,
	/// Database engine the instance is running
	pub engine: DatabaseEngine,
	/// Version of the database engine
	pub version: String,
	/// Number of database instances supposed to be running
	pub num_nodes: u16,
	/// Database plan ID selected by the user
	pub database_plan_id: Uuid,
	/// The region the database is deployed on
	pub region: Uuid,
	/// The current status of the database
	pub status: DatabaseStatus,
	/// The connection configuration for the user to connect to the database instance
	pub public_connection: DatabaseConnection,
}
