use std::{fmt::Display, str::FromStr};

use clap::{crate_authors, crate_description, crate_name, crate_version};
use eve_rs::AsError;
use semver::Version;

use crate::{error, utils::Error};

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

pub const PORTUS_DOCKER_IMAGE: &str = "portus_image:1.0";

pub enum AccountType {
	Personal,
	Organisation,
}

impl Display for AccountType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AccountType::Personal => write!(f, "personal"),
			AccountType::Organisation => write!(f, "organisation"),
		}
	}
}

impl FromStr for AccountType {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"personal" => Ok(AccountType::Personal),
			"organisation" => Ok(AccountType::Organisation),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

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
	pub const ORGANISATIONS: &str = "organisations";
	pub const NAME: &str = "name";
	pub const ACTIVE: &str = "active";
	pub const CREATED: &str = "created";
	pub const DOMAINS: &str = "domains";
	pub const VERIFIED: &str = "verified";
	pub const ID: &str = "id";
	pub const APPLICATIONS: &str = "applications";
	pub const APPLICATION_ID: &str = "applicationId";
	pub const VERSIONS: &str = "versions";
	pub const VERSION: &str = "version";
	pub const ROLE_ID: &str = "roleId";
	pub const ROLES: &str = "roles";
	pub const DESCRIPTION: &str = "description";
	pub const RESOURCE_PERMISSIONS: &str = "resourcePermissions";
	pub const RESOURCE_TYPE_PERMISSIONS: &str = "resourceTypePermissions";
	pub const PERMISSIONS: &str = "permissions";
	pub const PERMISSION_ID: &str = "permissionId";
	pub const RESOURCE: &str = "resource";
	pub const RESOURCE_ID: &str = "resourceId";
	pub const RESOURCE_TYPE_ID: &str = "resourceTypeId";
	pub const RESOURCE_TYPES: &str = "resourceTypes";
	pub const RESOURCE_TYPE: &str = "resourceType";
	pub const LOCAL_PORT: &str = "localPort";
	pub const LOCAL_HOST_NAME: &str = "localHostName";
	pub const EXPOSED_SERVER_PORT: &str = "exposedServerPort";
	pub const SERVER_IP: &str = "serverIp";
	pub const SERVER_USER_NAME: &str = "serverUserName";
	pub const SCRIPT: &str = "script";
	pub const SSH_PORT: &str = "sshPort";
	pub const EXPOSED_PORT: &str = "exposedPort";
	pub const SERVER_SSH_PORT: &str = "serverSSHPort";
	pub const TUNNEL_ID: &str = "tunnelId";
	pub const TUNNELS: &str = "tunnels";
	pub const LOGIN_ID: &str = "loginId";
}
