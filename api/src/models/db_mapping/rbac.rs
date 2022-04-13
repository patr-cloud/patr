use api_models::utils::Uuid;

pub struct Resource {
	pub id: Uuid,
	pub name: String,
	pub resource_type_id: Uuid,
	pub owner_id: Uuid,
	pub created: u64,
}

pub struct Role {
	pub id: Uuid,
	pub name: String,
	pub description: String,
	pub owner_id: Uuid,
}

pub struct Permission {
	pub id: Uuid,
	pub name: String,
	pub description: String,
}

pub struct ResourceType {
	pub id: Uuid,
	pub name: String,
	pub description: String,
}

pub struct WorkspaceUser {
	pub user_id: Uuid,
	pub workspace_id: Uuid,
	pub role_id: Uuid,
}
