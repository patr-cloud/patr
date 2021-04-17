pub struct User {
	pub id: Vec<u8>,
	pub username: String,
	pub password: String,
	pub backup_email: String,
	pub first_name: String,
	pub last_name: String,
	pub dob: Option<u64>,
	pub bio: Option<String>,
	pub location: Option<String>,
	pub created: u64,
}

pub struct UserLogin {
	pub login_id: Vec<u8>,
	pub refresh_token: String,
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
	pub password: String,
	pub otp_hash: String,
	pub otp_expiry: u64,
	pub first_name: String,
	pub last_name: String,
}

pub struct PasswordResetRequest {
	pub user_id: Vec<u8>,
	pub token: String,
	pub token_expiry: u64,
}

pub struct PersonalEmailToBeVerified {
	pub email_address: String,
	pub user_id: Vec<u8>,
	pub verification_token_hash: Vec<u8>,
	pub verification_token_expiry: u64,
}
