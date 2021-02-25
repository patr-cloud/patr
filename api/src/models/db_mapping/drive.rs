pub struct DriveFile {
	pub id: Vec<u8>,
	pub filename: String,
	pub folder_id: Option<Vec<u8>>,
	pub collection_id: Option<Vec<u8>>,
	pub created: u64,
}