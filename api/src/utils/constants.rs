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
	pub const DOMAIN: &str = "domain";
	pub const WORKSPACE_ID: &str = "workspaceId";
	pub const NAME: &str = "name";
	pub const ID: &str = "id";
	pub const VERSION: &str = "version";
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
	pub const PAYMENT_METHOD_ID: &str = "paymentMethodId";
}

pub mod default_limits {
	pub const DEPLOYMENTS: i32 = 10;
	pub const MANAGED_DATABASE: i32 = 5;
	pub const STATIC_SITES: i32 = 30;
	pub const MANAGED_URLS: i32 = 200;
	pub const DOCKER_REPOSITORY_STORAGE: i32 = 200;
	pub const DOMAINS: i32 = 5;
	pub const SECRETS: i32 = 150;
}
