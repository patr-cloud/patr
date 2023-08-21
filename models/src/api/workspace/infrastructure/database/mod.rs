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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedDatabaseConnection {
	pub host: String,
	pub port: u32,
	pub username: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedDatabase {
	pub id: Uuid,
	pub name: String,
	pub database_name: String,
	pub engine: String,
	pub version: String,
	pub num_nodes: u16,
	pub database_plan: String,
	pub region: String,
	pub status: String,
	pub public_connection: ManagedDatabaseConnection,
}
