use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};

pub struct DeploymentStaticSite {
	pub id: Uuid,
	pub name: String,
	pub status: DeploymentStatus,
	pub workspace_id: Uuid,
}
