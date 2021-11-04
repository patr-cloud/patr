use std::{fmt::Display, str::FromStr};

use api_macros::version;
use clap::{crate_authors, crate_description, crate_name, crate_version};
use eve_rs::AsError;
use semver::Version;

use crate::{error, utils::Error};

pub const DATABASE_VERSION: Version = version!();

pub const APP_NAME: &str = crate_name!();
pub const APP_VERSION: &str = crate_version!();
pub const APP_AUTHORS: &str = crate_authors!();
pub const APP_ABOUT: &str = crate_description!();

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "RESOURCE_OWNER_TYPE", rename_all = "lowercase")]
pub enum ResourceOwnerType {
	Personal,
	Business,
}

impl Display for ResourceOwnerType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ResourceOwnerType::Personal => write!(f, "personal"),
			ResourceOwnerType::Business => write!(f, "business"),
		}
	}
}

impl FromStr for ResourceOwnerType {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"personal" => Ok(Self::Personal),
			"business" => Ok(Self::Business),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[allow(dead_code)]
pub mod request_keys {
	pub const USER_ID: &str = "userId";
	pub const USERNAME: &str = "username";
	pub const EMAIL: &str = "email";
	pub const PASSWORD: &str = "password";
	pub const NEW_PASSWORD: &str = "newPassword";
	pub const SUCCESS: &str = "success";
	pub const ERROR: &str = "error";
	pub const ERRORS: &str = "errors";
	pub const MESSAGE: &str = "message";
	pub const ACCESS_TOKEN: &str = "accessToken";
	pub const REFRESH_TOKEN: &str = "refreshToken";
	pub const VERIFICATION_TOKEN: &str = "verificationToken";
	pub const CODE: &str = "code";
	pub const DETAIL: &str = "detail";
	pub const TOKEN: &str = "token";
	pub const AVAILABLE: &str = "available";
	pub const FIRST_NAME: &str = "firstName";
	pub const LAST_NAME: &str = "lastName";
	pub const ACCOUNT_TYPE: &str = "accountType";
	pub const DOMAIN: &str = "domain";
	pub const DOMAIN_ID: &str = "domainId";
	pub const WORKSPACE_NAME: &str = "workspaceName";
	pub const BUSINESS_EMAIL_LOCAL: &str = "businessEmailLocal";
	pub const BACKUP_EMAIL: &str = "backupEmail";
	pub const EMAILS: &str = "emails";
	pub const BACKUP_PHONE_COUNTRY_CODE: &str = "backupPhoneCountryCode";
	pub const BACKUP_PHONE_NUMBER: &str = "backupPhoneNumber";
	pub const PHONE_NUMBERS: &str = "phoneNumbers";
	pub const COUNTRY_CODE: &str = "countryCode";
	pub const PHONE_NUMBER: &str = "phoneNumber";
	pub const BIRTHDAY: &str = "birthday";
	pub const BIO: &str = "bio";
	pub const LOCATION: &str = "location";
	pub const WORKSPACE_ID: &str = "workspaceId";
	pub const WORKSPACES: &str = "workspaces";
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
	pub const SCOPE: &str = "scope";
	pub const SERVICE: &str = "service";
	pub const SNAKE_CASE_CLIENT_ID: &str = "client_id";
	pub const SNAKE_CASE_OFFLINE_TOKEN: &str = "offline_token";
	pub const REPOSITORY: &str = "repository";
	pub const REPOSITORIES: &str = "repositories";
	pub const REPOSITORY_ID: &str = "repositoryId";
	pub const DEPLOYMENT_ID: &str = "deploymentId";
	pub const DEPLOYMENTS: &str = "deployments";
	pub const DEPLOYMENT: &str = "deployment";
	pub const REGISTRY: &str = "registry";
	pub const IMAGE_NAME: &str = "imageName";
	pub const IMAGE_TAG: &str = "imageTag";
	pub const SUB_DOMAIN: &str = "subDomain";
	pub const PATH: &str = "path";
	pub const ENVIRONMENT_VARIABLES: &str = "environmentVariables";
	pub const PERSISTENT_VOLUMES: &str = "volumes";
	pub const EXPOSED_PORTS: &str = "ports";
	pub const UPGRADE_PATH_ID: &str = "upgradePathId";
	pub const UPGRADE_PATHS: &str = "upgradePaths";
	pub const MACHINE_TYPES: &str = "machineTypes";
	pub const ENTRY_POINT_ID: &str = "entryPointId";
	pub const ENTRY_POINTS: &str = "entryPoints";
	pub const ENTRY_POINT_TYPE: &str = "entryPointType";
	pub const PORT: &str = "port";
	pub const PREFERRED_RECOVERY_OPTION: &str = "preferredRecoveryOption";
	pub const URL: &str = "url";
	pub const DEFAULT: &str = "default";
	pub const TOKEN_EXPIRY: &str = "tokenExpiry";
	pub const LAST_LOGIN: &str = "lastLogin";
	pub const LAST_ACTIVITY: &str = "lastActivity";
	pub const LOGINS: &str = "logins";
	pub const STATUS: &str = "status";
	pub const ENGINE: &str = "engine";
	pub const NUM_NODES: &str = "numNodes";
	pub const REGION: &str = "region";
	pub const URI: &str = "uri";
	pub const HOST: &str = "host";
	pub const CREATED_AT: &str = "createdAt";
	pub const DATABASES: &str = "databases";
	pub const DATABASE_PLAN: &str = "databasePlan";
	pub const PUBLIC_CONNECTION: &str = "publicConnection";
	pub const LOGS: &str = "logs";
	pub const DATABASE_NAME: &str = "databaseName";
	pub const DATABASE_ID: &str = "databaseId";
	pub const CNAME: &str = "cname";
	pub const VALUE: &str = "value";
	pub const CNAME_RECORDS: &str = "cnameRecords";
	pub const DOMAIN_NAME: &str = "domainName";
	pub const VALIDATED: &str = "validated";
	pub const HORIZONTAL_SCALE: &str = "horizontalScale";
	pub const MACHINE_TYPE: &str = "machineType";
	pub const STATIC_SITES: &str = "staticSites";
	pub const STATIC_SITE_ID: &str = "staticSiteId";
	pub const STATIC_SITE_FILE: &str = "staticSiteFile";
	pub const IP_ADDRESS: &str = "ipAddress";
	pub const IP_ADDRESS_LATITUDE: &str = "ipAddressLatitude";
	pub const IP_ADDRESS_LONGITUDE: &str = "ipAddressLongitude";
	pub const METHOD: &str = "method";
	pub const PROTOCOL: &str = "protocol";
	pub const RESPONSE_TIME: &str = "responseTime";
	pub const DISTANCE: &str = "distance";
	pub const RECOMMENDED_DATA_CENTER: &str = "recommendedDataCenter";
	pub const SECONDARY_EMAILS: &str = "secondaryEmails";
	pub const SECONDARY_PHONE_NUMBERS: &str = "secondaryPhoneNumbers";
	pub const USERS_TO_SIGN_UP: &str = "usersToSignUp";
	pub const USERS: &str = "users";
	pub const CUSTOM_DOMAINS_FOR_DEPLOYMENTS: &str =
		"customDomainsForDeployments";
	pub const DELETED_DEPLOYMENTS: &str = "deletedDeployments";
	pub const DELETED_DATABASES: &str = "deletedDatabases";
	pub const CUSTOM_DOMAINS_FOR_STATIC_SITES: &str =
		"customDomainsForStaticSites";
	pub const DELETED_STATIC_SITES: &str = "deletedStaticSites";
	pub const TOTAL_WEBSITES: &str = "totalWebsites";
	pub const TOTAL_RESOURCES: &str = "totalResources";
}
