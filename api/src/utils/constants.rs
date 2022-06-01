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
	pub const SUCCESS: &str = "success";
	pub const ERROR: &str = "error";
	pub const ERRORS: &str = "errors";
	pub const MESSAGE: &str = "message";
	pub const CODE: &str = "code";
	pub const DETAIL: &str = "detail";
	pub const TOKEN: &str = "token";
	pub const DOMAIN_ID: &str = "domainId";
	pub const WORKSPACE_ID: &str = "workspaceId";
	pub const ROLE_ID: &str = "roleId";
	pub const RESOURCE_ID: &str = "resourceId";
	pub const LOGIN_ID: &str = "loginId";
	pub const SCOPE: &str = "scope";
	pub const SERVICE: &str = "service";
	pub const SNAKE_CASE_CLIENT_ID: &str = "client_id";
	pub const SNAKE_CASE_OFFLINE_TOKEN: &str = "offline_token";
	pub const REPOSITORY: &str = "repository";
	pub const REPOSITORY_ID: &str = "repositoryId";
	pub const DEPLOYMENT_ID: &str = "deploymentId";
	pub const REGION: &str = "region";
	pub const DATABASES: &str = "databases";
	pub const DATABASE_ID: &str = "databaseId";
	pub const STATIC_SITE_ID: &str = "staticSiteId";
	pub const USER_ID: &str = "userId";
	pub const RECORD_ID: &str = "recordId";
	pub const DIGEST: &str = "digest";
	pub const TAG: &str = "tag";
	pub const MANAGED_URL_ID: &str = "managedUrlId";
	pub const START_TIME: &str = "startTime";
	pub const INTERVAL: &str = "interval";
	pub const SECRET_ID: &str = "secretId";
	pub const REPO_OWNER: &str = "repoOwner";
	pub const REPO_NAME: &str = "repoName";
	pub const BUILD_NUM: &str = "buildNum";
	pub const STAGE: &str = "stage";
	pub const STEP: &str = "step";
}
