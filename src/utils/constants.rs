use semver::Version;

pub const DATABASE_VERSION: Version = Version {
	major: 0,
	minor: 0,
	patch: 0,
	pre: vec![],
	build: vec![],
};

pub mod request_keys {
	pub const USER_ID: &str = "userId";
	pub const USERNAME: &str = "username";
	pub const EMAIL: &str = "email";
	pub const PASSWORD: &str = "password";
	pub const SUCCESS: &str = "success";
	pub const ERROR: &str = "error";
	pub const MESSAGE: &str = "message";
	pub const ACCESS_TOKEN: &str = "accessToken";
	pub const REFRESH_TOKEN: &str = "refreshToken";
	pub const VERIFICATION_TOKEN: &str = "verificationToken";
	pub const AVAILABLE: &str = "available";
	pub const PHONE_NUMBER: &str = "phoneNumber";
	pub const FIRST_NAME: &str = "firstName";
	pub const LAST_NAME: &str = "lastName";
}
