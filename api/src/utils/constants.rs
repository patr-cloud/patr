use clap::{crate_authors, crate_description, crate_name, crate_version};
use semver::Version;

pub const DATABASE_VERSION: Version = Version {
	major: 0,
	minor: 0,
	patch: 0,
	pre: vec![],
	build: vec![],
};

pub const APP_NAME: &str = crate_name!();
pub const APP_VERSION: &str = crate_version!();
pub const APP_AUTHORS: &str = crate_authors!();
pub const APP_ABOUT: &str = crate_description!();

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
	pub const FIRST_NAME: &str = "firstName";
	pub const LAST_NAME: &str = "lastName";
	pub const ACCOUNT_TYPE: &str = "accountType";
	pub const DOMAIN: &str = "domain";
	pub const DOMAIN_ID: &str = "domainId";
	pub const ORGANISATION_NAME: &str = "organisationName";
	pub const BACKUP_EMAIL: &str = "backupEmail";
	pub const BIRTHDAY: &str = "birthday";
	pub const BIO: &str = "bio";
	pub const LOCATION: &str = "location";
	pub const ORGANISATION_ID: &str = "organisationId";
	pub const NAME: &str = "name";
	pub const ACTIVE: &str = "active";
	pub const CREATED: &str = "created";
	pub const DOMAINS: &str = "domains";
	pub const VERIFIED: &str = "verified";
	pub const ID: &str = "id";
}
