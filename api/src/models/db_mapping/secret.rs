use api_models::utils::Uuid;

pub struct Secret {
	pub id: Uuid,
	pub name: String,
	pub workspace_id: Uuid,
	pub deployment_id: Option<Uuid>,
}
