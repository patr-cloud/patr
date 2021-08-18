pub struct Resource {
	pub id: Vec<u8>,
	pub name: String,
	pub resource_type_id: Vec<u8>,
	pub owner_id: Vec<u8>,
	pub created: u64,
}

pub struct Role {
	pub id: Vec<u8>,
	pub name: String,
	pub description: Option<String>,
	pub owner_id: Vec<u8>,
}

pub struct Permission {
	pub id: Vec<u8>,
	pub name: String,
	pub description: Option<String>,
}

pub struct ResourceType {
	pub id: Vec<u8>,
	pub name: String,
	pub description: Option<String>,
}
