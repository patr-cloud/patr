pub struct User {
	pub user_id: Vec<u8>,
	pub username: String,
	pub password: Vec<u8>,
	pub email: String,
}

impl User {
	pub fn from(
		user_id: Vec<u8>,
		username: String,
		password: Vec<u8>,
		email: String,
	) -> User {
		User {
			user_id: user_id.to_vec(),
			username,
			password: password.to_vec(),
			email,
		}
	}
}
