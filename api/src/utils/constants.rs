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

#[derive(sqlx::Type, Debug, PartialEq)]
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

pub mod request_keys {
	pub const USERNAME: &str = "username";
	pub const PASSWORD: &str = "password";
	pub const SUCCESS: &str = "success";
	pub const ERROR: &str = "error";
	pub const ERRORS: &str = "errors";
	pub const MESSAGE: &str = "message";
	pub const VERIFICATION_TOKEN: &str = "verificationToken";
	pub const CODE: &str = "code";
	pub const DETAIL: &str = "detail";
	pub const TOKEN: &str = "token";
	pub const DOMAIN: &str = "domain";
	pub const DOMAIN_ID: &str = "domainId";
	pub const WORKSPACE_ID: &str = "workspaceId";
	pub const NAME: &str = "name";
	pub const DOMAINS: &str = "domains";
	pub const VERIFIED: &str = "verified";
	pub const ID: &str = "id";
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
	pub const LOGIN_ID: &str = "loginId";
	pub const SCOPE: &str = "scope";
	pub const SERVICE: &str = "service";
	pub const SNAKE_CASE_CLIENT_ID: &str = "client_id";
	pub const SNAKE_CASE_OFFLINE_TOKEN: &str = "offline_token";
	pub const REPOSITORY: &str = "repository";
	pub const REPOSITORY_ID: &str = "repositoryId";
	pub const DEPLOYMENT_ID: &str = "deploymentId";
	pub const DEPLOYMENTS: &str = "deployments";
	pub const PORT: &str = "port";
	pub const STATUS: &str = "status";
	pub const ENGINE: &str = "engine";
	pub const NUM_NODES: &str = "numNodes";
	pub const REGION: &str = "region";
	pub const HOST: &str = "host";
	pub const DATABASES: &str = "databases";
	pub const DATABASE_PLAN: &str = "databasePlan";
	pub const PUBLIC_CONNECTION: &str = "publicConnection";
	pub const DATABASE_NAME: &str = "databaseName";
	pub const DATABASE_ID: &str = "databaseId";
	pub const STATIC_SITES: &str = "staticSites";
	pub const STATIC_SITE_ID: &str = "staticSiteId";
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
	pub const DIGEST: &str = "digest";
	pub const TAG: &str = "tag";
	pub const MANAGED_URL_ID: &str = "managedUrlId";
	pub const DEPLOYMENT_REGIONS: [(&str, &str, (f64, f64)); 21] = [
		("Asia", "", (0.0, 0.0)),
		("Europe", "", (0.0, 0.0)),
		("North-America", "", (0.0, 0.0)),
		("South-America", "", (0.0, 0.0)),
		("Australia", "", (0.0, 0.0)),
		("Africa", "", (0.0, 0.0)),
		("Asia::India", "", (0.0, 0.0)),
		("Asia::Singapore", "digitalocean", (1.3521, 103.8198)),
		("Europe::England", "", (0.0, 0.0)),
		("Europe::Netherlands", "", (0.0, 0.0)),
		("Europe::Germany", "", (0.0, 0.0)),
		("North-America::Canada", "", (0.0, 0.0)),
		("North-America::USA", "", (0.0, 0.0)),
		("Asia::India::Bangalore", "digitalocean", (2.9716, 77.5946)),
		("Europe::England::London", "digtalocean", (51.5072, 0.1276)),
		(
			"Europe::Netherlands::Amsterdam",
			"digitalocean",
			(52.3676, 4.9041),
		),
		(
			"Europe::Germany::Frankfurt",
			"digitalocean",
			(50.1109, 8.6821),
		),
		(
			"North-America::Canada::Toronto",
			"digitalocean",
			(43.6532, 79.3832),
		),
		(
			"North-America::USA::New-York-1",
			"digitalocean",
			(40.7128, 74.0060),
		),
		(
			"North-America::USA::New-York-2",
			"digitalocean",
			(40.7128, 74.0060),
		),
		(
			"North-America::USA::San-Francisco",
			"digitalocean",
			(37.7749, 122.4194),
		),
	];
}
