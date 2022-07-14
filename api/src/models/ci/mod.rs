use serde::{Deserialize, Serialize};

pub mod file_format;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DroneUserInfoResponse {
	pub login: String,
	pub token: String,
}
