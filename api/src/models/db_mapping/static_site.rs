use uuid::Uuid;

use super::DeploymentStatus;

pub struct DeploymentStaticSite {
	pub id: Uuid,
	pub name: String,
	pub status: DeploymentStatus,
	pub domain_name: Option<String>,
	pub workspace_id: Uuid,
}
