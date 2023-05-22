use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatefulSetConfig {
	pub secret_name_for_db_pwd: String,
	pub svc_name_for_db: String,
	pub sts_name_for_db: String,
	pub pvc_claim_for_db: String,
}
