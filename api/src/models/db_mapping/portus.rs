pub struct PortusTunnel {
	pub id: Vec<u8>,
	pub username: String,
	pub ssh_port: u16,
	pub exposed_port: u16,
	pub created: u64,
	pub tunnel_name: String,
}
