use serde::{Deserialize, Serialize};

/// TODO doc
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IaacDatabase {
	/// TODO doc
	pub name: String,
	/// TODO doc
	#[serde(alias = "dbEngine")]
	pub engine: IaacDatabaseEngine,
	/// TODO doc
	#[serde(skip_serializing_if = "Option::is_none")]
	pub save_password_to: Option<String>,
}

/// TODO doc
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub enum IaacDatabaseEngine {
	/// TODO doc
	#[serde(alias = "postgres", alias = "postgresql")]
	Postgres,
	/// TODO doc
	#[serde(alias = "mysql")]
	MySQL,
	/// TODO doc
	#[serde(alias = "mongodb", alias = "mongo")]
	MongoDB,
	/// TODO doc
	#[serde(alias = "redis")]
	Redis,
}
