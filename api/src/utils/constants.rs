use api_macros::version;
use semver::Version;

pub const DATABASE_VERSION: Version = version!();

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
	pub const REGION_ID: &str = "regionId";
	pub const DATABASES: &str = "databases";
	pub const DATABASE_ID: &str = "databaseId";
	pub const STATIC_SITE_ID: &str = "staticSiteId";
	pub const UPLOAD_ID: &str = "uploadId";
	pub const USER_ID: &str = "userId";
	pub const RECORD_ID: &str = "recordId";
	pub const DIGEST: &str = "digest";
	pub const TAG: &str = "tag";
	pub const MANAGED_URL_ID: &str = "managedUrlId";
	pub const START_TIME: &str = "startTime";
	pub const INTERVAL: &str = "interval";
	pub const SECRET_ID: &str = "secretId";
	pub const REPO_ID: &str = "repoId";
	pub const BUILD_NUM: &str = "buildNum";
	pub const BRANCH_NAME: &str = "branchName";
	pub const GIT_REF: &str = "gitRef";
	pub const STEP: &str = "step";
	pub const PAYMENT_METHOD_ID: &str = "paymentMethodId";
	pub const TOKEN_ID: &str = "tokenId";
	// github constants for CI
	pub const X_HUB_SIGNATURE_256: &str = "x-hub-signature-256";
	pub const X_GITHUB_EVENT: &str = "x-github-event";
	pub const RUNNER_ID: &str = "runnerId";
}

pub mod default_limits {
	pub const DEPLOYMENTS: i32 = 10;
	pub const MANAGED_DATABASE: i32 = 5;
	pub const STATIC_SITES: i32 = 30;
	pub const MANAGED_URLS: i32 = 200;
	pub const DOCKER_REPOSITORY_STORAGE: i32 = 200;
	pub const DOMAINS: i32 = 5;
	pub const SECRETS: i32 = 150;
	pub const VOLUME_STORAGE: i32 = 100;
}

pub mod free_limits {
	pub const DEPLOYMENT_COUNT: usize = 1;
	pub const MANAGED_DATABASE_COUNT: usize = 0;
	pub const STATIC_SITE_COUNT: usize = 3;
	pub const MANAGED_URL_COUNT: usize = 10;
	pub const DOMAIN_COUNT: usize = 1;
	pub const SECRET_COUNT: usize = 3;
	pub const DOCKER_REPOSITORY_STORAGE_IN_BYTES: usize =
		10 * 1024 * 1024 * 1024; // 10 GB
	pub const VOLUME_STORAGE_IN_BYTE: usize = 0; // 0 GB - No free volumes
}

pub mod github_oauth {
	pub const SCOPE: &str = "user";
	pub const AUTH_URL: &str = "https://github.com/login/oauth/authorize";
	pub const CALLBACK_URL: &str =
		"https://github.com/login/oauth/access_token";
	pub const USER_INFO_API: &str = "https://api.github.com/user";
	pub const USER_EMAIL_API: &str = "https://api.github.com/user/emails";
}

pub const PATR_CLUSTER_TENANT_ID: &str = "patr-internal";
pub const PATR_BYOC_TOKEN_NAME: &str = "patr-token";
pub const PATR_BYOC_TOKEN_VALUE_NAME: &str = "token";
