pub struct User {
	pub id: Vec<u8>,
	pub username: String,
	pub password: Vec<u8>,
	pub backup_email: String,
	pub first_name: String,
	pub last_name: String,
	pub created: u64,
}

pub struct UserLogin {
	pub refresh_token: Vec<u8>,
	pub token_expiry: u64,
	pub user_id: Vec<u8>,
	pub last_login: u64,
	pub last_activity: u64,
}

#[derive(Clone)]
pub enum UserEmailAddress {
	Personal(String),
	Organisation {
		email_local: String,
		domain_id: Vec<u8>,
	},
}

#[derive(Clone)]
pub enum UserEmailAddressSignUp {
	Personal(String),
	Organisation {
		email_local: String,
		domain_name: String,
		organisation_name: String,
		backup_email: String,
	},
}

pub struct UserToSignUp {
	pub username: String,
	pub backup_email: String,
	pub email: UserEmailAddressSignUp,
	pub password: Vec<u8>,
	pub otp_hash: Vec<u8>,
	pub otp_expiry: u64,
	pub first_name: String,
	pub last_name: String,
}
