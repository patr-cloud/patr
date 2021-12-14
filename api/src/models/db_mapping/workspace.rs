use api_models::utils::Uuid;

use crate::utils::constants::ResourceOwnerType;

pub struct Workspace {
	pub id: Uuid,
	pub name: String,
	pub super_admin_id: Uuid,
	pub active: bool,
}

pub struct Domain {
	pub id: Uuid,
	pub name: String,
	pub r#type: ResourceOwnerType,
}

pub struct PersonalDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceOwnerType,
}

pub struct WorkspaceDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceOwnerType,
	pub is_verified: bool,
}
