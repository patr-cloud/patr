use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentStaticSite {
	pub id: Uuid,
	pub name: String,
	pub status: DeploymentStatus,
	pub workspace_id: Uuid,
}
