use crate::utils::constants::ResourceOwnerType;

pub struct Workspace {
	pub id: Vec<u8>,
	pub name: String,
	pub super_admin_id: Vec<u8>,
	pub active: bool,
}

pub struct Domain {
	pub id: Vec<u8>,
	pub name: String,
	pub r#type: ResourceOwnerType,
}

pub struct PersonalDomain {
	pub id: Vec<u8>,
	pub name: String,
	pub domain_type: ResourceOwnerType,
}

pub struct WorkspaceDomain {
	pub id: Vec<u8>,
	pub name: String,
	pub domain_type: ResourceOwnerType,
	pub is_verified: bool,
}
