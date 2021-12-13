use uuid::Uuid;

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
	pub description: Option<String>,
	pub owner_id: Uuid,
}

pub struct Permission {
	pub id: Uuid,
	pub name: String,
	pub description: Option<String>,
}

pub struct ResourceType {
	pub id: Uuid,
	pub name: String,
	pub description: Option<String>,
}
