use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

pub struct Secret {
	pub id: Uuid,
	pub name: String,
	pub workspace_id: Uuid,
	pub deployment_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecretBody {
	pub name: String,
	pub secret: String,
}
