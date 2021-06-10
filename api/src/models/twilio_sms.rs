use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SmsRequest {
	pub body: String,
	pub from: String,
	pub to: String,
}

#[derive(Debug, Deserialize)]
pub struct SmsResponse {
	pub status: String,
}
