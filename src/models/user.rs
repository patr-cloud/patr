pub struct User {
	pub id: Vec<u8>,
	pub username: String,
	pub password: Vec<u8>,
	pub email: String,
}

impl User {
	pub fn from(id: Vec<u8>, username: String, password: Vec<u8>, email: String) -> Self {
		Self {
			id,
			username,
			password,
			email,
		}
	}
}

pub struct UserLogin {
	pub refresh_token: Vec<u8>,
	pub token_expiry: u64,
	pub user_id: Vec<u8>,
	pub last_login: u64,
	pub last_activity: u64,
}

impl UserLogin {
	pub fn from(
		refresh_token: Vec<u8>,
		token_expiry: u64,
		user_id: Vec<u8>,
		last_login: u64,
		last_activity: u64,
	) -> Self {
		Self {
			refresh_token,
			token_expiry,
			user_id,
			last_login,
			last_activity,
		}
	}
}
