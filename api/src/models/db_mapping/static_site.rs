use super::DeploymentStatus;
pub struct DeploymentStaticSite {
	pub id: Vec<u8>,
	pub name: String,
	pub status: DeploymentStatus,
	pub domain_name: Option<String>,
	pub organisation_id: Vec<u8>,
}
