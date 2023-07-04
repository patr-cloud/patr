use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSecretResponse {
	pub data: Value,
	pub auth: Option<String>,
	pub lease_duration: i32,
	pub lease_id: String,
	pub renewable: bool,
	pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultResponse {
	pub data: VaultResponseData,
	pub metadata: VaultResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultResponseData {
	pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultResponseMetadata {
	pub created_time: DateTime<Utc>,
	pub custom_metadata: Option<String>,
	pub deletion_time: String,
	pub destroyed: bool,
	pub version: i32,
}
