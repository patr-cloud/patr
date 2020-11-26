pub struct Organisation {
	pub id: Vec<u8>,
	pub name: String,
	pub super_admin_id: Vec<u8>,
	pub active: bool,
	pub created: u64,
}

pub struct Domain {
	pub id: Vec<u8>,
	pub name: String,
	pub is_verified: bool,
}
