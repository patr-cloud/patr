use api_macros::version;
use clap::{crate_authors, crate_description, crate_name, crate_version};
use semver::Version;

pub const DATABASE_VERSION: Version = version!();

pub const APP_NAME: &str = crate_name!();
pub const APP_VERSION: &str = crate_version!();
pub const APP_AUTHORS: &str = crate_authors!();
pub const APP_ABOUT: &str = crate_description!();

pub const DNS_RESOLVER: &str = "1.1.1.1:53";

pub mod request_keys {
	pub const USERNAME: &str = "username";
	pub const PASSWORD: &str = "password";
	pub const SUCCESS: &str = "success";
	pub const ERROR: &str = "error";
	pub const ERRORS: &str = "errors";
	pub const MESSAGE: &str = "message";
	pub const CODE: &str = "code";
	pub const DETAIL: &str = "detail";
	pub const TOKEN: &str = "token";
	pub const DOMAIN_ID: &str = "domainId";
	pub const WORKSPACE_ID: &str = "workspaceId";
	pub const NAME: &str = "name";
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
	pub const USER_ID: &str = "userId";
	pub const CUSTOM_DOMAINS_FOR_DEPLOYMENTS: &str =
		"customDomainsForDeployments";
	pub const DELETED_DEPLOYMENTS: &str = "deletedDeployments";
	pub const DELETED_DATABASES: &str = "deletedDatabases";
	pub const CUSTOM_DOMAINS_FOR_STATIC_SITES: &str =
		"customDomainsForStaticSites";
	pub const DELETED_STATIC_SITES: &str = "deletedStaticSites";
	pub const TOTAL_WEBSITES: &str = "totalWebsites";
	pub const TOTAL_RESOURCES: &str = "totalResources";
	pub const RECORD_ID: &str = "recordId";
	pub const DIGEST: &str = "digest";
	pub const TAG: &str = "tag";
	pub const MANAGED_URL_ID: &str = "managedUrlId";
}
