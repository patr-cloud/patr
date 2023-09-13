use serde::{Deserialize, Serialize};

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IaacDatabase {
	pub name: String,
	#[serde(alias = "dbEngine")]
	pub engine: IaacDatabaseEngine,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub save_password_to: Option<String>,
}

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(deny_unknown_fields)]
pub enum IaacDatabaseEngine {
	#[serde(alias = "postgres", alias = "postgresql")]
	Postgres,
	#[serde(alias = "mysql")]
	MySQL,
	#[serde(alias = "mongodb", alias = "mongo")]
	MongoDB,
	#[serde(alias = "redis")]
	Redis,
}
