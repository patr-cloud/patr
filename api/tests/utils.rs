use std::collections::BTreeMap;

use headers::UserAgent;
use models::{
	api::{
		auth::*,
		user::*,
		workspace::{
			container_registry::*,
			deployment::*,
			domain::*,
			managed_url::*,
			rbac::{role::*, user::*},
			runner::*,
			volume::*,
			*,
		},
	},
	rbac::{Permission, ResourcePermissionType, WorkspacePermission},
	utils::{BearerToken, Uuid},
};
use rand::RngExt as _;

use crate::prelude::*;

/// The User-Agent header value to use for all test API calls, which includes
/// the cargo-test identifier and the current package version.
pub const TEST_USER_AGENT: UserAgent = UserAgent::from_static(concat!(
	"cargo-test/",
	env!("CARGO_PKG_VERSION_MAJOR"),
	".",
	env!("CARGO_PKG_VERSION_MINOR"),
	".",
	env!("CARGO_PKG_VERSION_PATCH"),
));

/// A test user with credentials and tokens.
pub struct TestUser {
	pub user_id: Uuid,
	pub username: String,
	pub password: String,
	pub access_token: BearerToken,
	pub refresh_token: BearerToken,
}

/// A test workspace.
pub struct TestWorkspace {
	pub id: Uuid,
	pub name: String,
}

/// A test runner.
pub struct TestRunner {
	pub id: Uuid,
	pub name: String,
}

/// A test deployment.
pub struct TestDeployment {
	pub id: Uuid,
	pub name: String,
}

/// A test domain.
pub struct TestDomain {
	pub id: Uuid,
	pub domain: String,
}

/// A test volume.
pub struct TestVolume {
	pub id: Uuid,
	pub name: String,
}

/// A test container repository.
pub struct TestContainerRepo {
	pub id: Uuid,
	pub name: String,
}

/// A test role.
pub struct TestRole {
	pub id: Uuid,
	pub name: String,
}

/// A test API token.
pub struct TestApiToken {
	pub id: Uuid,
	pub token: String,
	pub name: String,
}

/// Generate a random lowercase alphanumeric string suitable for use as a
/// username or resource name.
pub fn random_name(len: usize) -> String {
	format!(
		"t{}",
		rand::rng()
			.sample_iter(rand::distr::Alphanumeric)
			.map(char::from)
			.take(len)
			.collect::<String>()
			.to_lowercase()
	)
}

/// Generate a random password that meets the validation requirements
/// (min 8 chars, uppercase, lowercase, digit, special char).
pub fn random_password() -> String {
	format!(
		"{}@1Aa",
		rand::rng()
			.sample_iter(rand::distr::Alphanumeric)
			.map(char::from)
			.take(28)
			.collect::<String>()
	)
}

impl TestSetup {
	/// Create a new test user account (CreateAccount + CompleteSignUp),
	/// returning the user's credentials and tokens.
	pub async fn create_test_user(&self) -> TestUser {
		let username = random_name(8);
		let password = random_password();

		self.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					username: username.clone(),
					password: password.clone(),
					first_name: "Test".to_string(),
					last_name: "User".to_string(),
					recovery_method: RecoveryMethod::Email {
						recovery_email: format!("{}@example.com", &username),
					},
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CompleteSignUpRequest>::builder()
					.headers(CompleteSignUpRequestHeaders {
						user_agent: TEST_USER_AGENT,
					})
					.body(CompleteSignUpRequest {
						username: username.clone(),
						verification_token: "000000".to_string(),
						cf_turnstile_token: "1x00000000000000000000AA".to_string(),
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CompleteSignUpResponse>>()
			.response;

		let user_info = self
			.make_web_dashboard_call(
				ApiRequest::<GetUserInfoRequest>::builder()
					.headers(GetUserInfoRequestHeaders {
						authorization: BearerToken::from_str(&response.access_token).unwrap(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

		self.clear_rate_limits().await;

		TestUser {
			user_id: user_info.response.basic_user_info.id,
			username,
			password,
			access_token: BearerToken::from_str(&response.access_token).unwrap(),
			refresh_token: BearerToken::from_str(&response.refresh_token).unwrap(),
		}
	}

	/// Login an existing test user, returning new access and refresh tokens.
	pub async fn login_test_user(&self, username: &str, password: &str) -> (String, String) {
		let response = self
			.make_web_dashboard_call(
				ApiRequest::<LoginRequest>::builder()
					.headers(LoginRequestHeaders {
						user_agent: TEST_USER_AGENT,
					})
					.body(LoginRequest {
						user_id: username.to_string(),
						password: password.to_string(),
						mfa_otp: None,
						cf_turnstile_token: "1x00000000000000000000AA".to_string(),
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<LoginResponse>>()
			.response;

		self.clear_rate_limits().await;

		(response.access_token, response.refresh_token)
	}

	/// Create a new workspace, returning its ID and name.
	pub async fn create_test_workspace(&self, token: &BearerToken) -> TestWorkspace {
		let name = random_name(8);

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CreateWorkspaceRequest>::builder()
					.headers(CreateWorkspaceRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateWorkspaceRequest { name: name.clone() })
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateWorkspaceResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestWorkspace {
			id: response.id.id,
			name,
		}
	}

	/// Add a runner to a workspace, returning its ID and name.
	pub async fn create_test_runner(&self, token: &BearerToken, workspace_id: Uuid) -> TestRunner {
		let name = random_name(8);

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
					.path(AddRunnerToWorkspacePath { workspace_id })
					.headers(AddRunnerToWorkspaceRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(AddRunnerToWorkspaceRequest { name: name.clone() })
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<AddRunnerToWorkspaceResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestRunner {
			id: response.id.id,
			name,
		}
	}

	/// Create a deployment using an external image (nginx), returning its ID
	/// and name.
	pub async fn create_test_deployment(
		&self,
		token: &BearerToken,
		workspace_id: Uuid,
		runner_id: Uuid,
	) -> TestDeployment {
		let name = random_name(8);

		// First get a valid machine type
		let machine_types = self
			.make_web_dashboard_call(
				ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
					.path(ListAllDeploymentMachineTypePath { workspace_id })
					.headers(ListAllDeploymentMachineTypeRequestHeaders {
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();

		let machine_type_id = machine_types
			.response
			.machine_types
			.first()
			.expect("no machine types available")
			.id;

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CreateDeploymentRequest>::builder()
					.path(CreateDeploymentPath { workspace_id })
					.headers(CreateDeploymentRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateDeploymentRequest {
						name: name.clone(),
						registry: DeploymentRegistry::ExternalRegistry {
							registry: "docker.io".to_string(),
							image_name: "library/nginx".to_string(),
						},
						image_tag: "latest".to_string(),
						runner: runner_id,
						machine_type: machine_type_id,
						running_details: DeploymentRunningDetails {
							deploy_on_push: false,
							min_horizontal_scale: 1,
							max_horizontal_scale: 1,
							ports: BTreeMap::new(),
							environment_variables: BTreeMap::new(),
							startup_probe: None,
							liveness_probe: None,
							config_mounts: BTreeMap::new(),
							volumes: BTreeMap::new(),
						},
						deploy_on_create: false,
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestDeployment {
			id: response.id.id,
			name,
		}
	}

	/// Force-mark a domain as verified by flipping the `is_verified` flag
	/// directly in the DB, skipping the real TXT-record + Cloudflare
	/// verification flow. Used by tests that depend on a verified domain (e.g.
	/// managed URL creation) but don't exercise the verification flow itself.
	pub async fn mark_test_domain_verified(&self, domain_id: Uuid) {
		query!(
			r#"
			UPDATE
				workspace_domain
			SET
				is_verified = TRUE,
				last_verified = NOW()
			WHERE
				id = $1;
			"#,
			domain_id as _,
		)
		.execute(self.database())
		.await
		.expect("failed to mark test domain as verified");
	}

	/// Add a domain to a workspace, returning its ID and domain name.
	pub async fn create_test_domain(&self, token: &BearerToken, workspace_id: Uuid) -> TestDomain {
		let domain = format!("{}.com", random_name(8));

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<AddDomainToWorkspaceRequest>::builder()
					.path(AddDomainToWorkspacePath { workspace_id })
					.headers(AddDomainToWorkspaceRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(AddDomainToWorkspaceRequest {
						domain: domain.clone(),
						nameserver_type: DomainNameserverType::External,
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<AddDomainToWorkspaceResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestDomain {
			id: response.id.id,
			domain,
		}
	}

	/// Create a volume in a workspace, returning its ID and name.
	pub async fn create_test_volume(&self, token: &BearerToken, workspace_id: Uuid) -> TestVolume {
		let name = random_name(8);

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CreateVolumeRequest>::builder()
					.path(CreateVolumePath { workspace_id })
					.headers(CreateVolumeRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateVolumeRequest {
						name: name.clone(),
						size: 1,
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateVolumeResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestVolume {
			id: response.id.id,
			name,
		}
	}

	/// Create a container repository in a workspace, returning its ID and name.
	pub async fn create_test_container_repo(
		&self,
		token: &BearerToken,
		workspace_id: Uuid,
	) -> TestContainerRepo {
		let name = random_name(8);

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CreateContainerRepositoryRequest>::builder()
					.path(CreateContainerRepositoryPath { workspace_id })
					.headers(CreateContainerRepositoryRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateContainerRepositoryRequest { name: name.clone() })
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateContainerRepositoryResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestContainerRepo {
			id: response.id.id,
			name,
		}
	}

	/// Create a role in a workspace with a minimal harmless permission
	/// (ViewRoles), returning its ID and name. The handler rejects empty
	/// permission maps with `WrongParameters`, so we always seed one.
	pub async fn create_test_role(&self, token: &BearerToken, workspace_id: Uuid) -> TestRole {
		// The `create_new_role` handler rejects empty permissions with
		// `WrongParameters`, so seed one harmless permission. Tests that care
		// about specific permissions should use `create_role_with_permissions`.
		let mut permissions = BTreeMap::new();
		permissions.insert(
			self.get_permission_id(Permission::ViewRoles),
			ResourcePermissionType::Include(Default::default()),
		);
		self.create_role_with_permissions(token, workspace_id, permissions)
			.await
	}

	/// Create a role with specific permissions.
	pub async fn create_role_with_permissions(
		&self,
		token: &BearerToken,
		workspace_id: Uuid,
		permissions: BTreeMap<Uuid, ResourcePermissionType>,
	) -> TestRole {
		let name = random_name(8);

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CreateNewRoleRequest>::builder()
					.path(CreateNewRolePath { workspace_id })
					.headers(CreateNewRoleRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateNewRoleRequest {
						name: name.clone(),
						description: "test role with permissions".to_string(),
						permissions,
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateNewRoleResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestRole {
			id: response.id.id,
			name,
		}
	}

	/// Create an API token for the user with the given workspace permissions,
	/// returning its ID, token string, and name.
	pub async fn create_test_api_token(
		&self,
		token: &BearerToken,
		permissions: BTreeMap<Uuid, WorkspacePermission>,
	) -> TestApiToken {
		let name = random_name(8);

		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CreateApiTokenRequest>::builder()
					.headers(CreateApiTokenRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateApiTokenRequest {
						token: UserApiToken {
							name: name.clone(),
							permissions,
							token_nbf: None,
							token_exp: None,
							allowed_ips: None,
							created: time::OffsetDateTime::now_utc(),
						},
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
			.response;

		self.clear_rate_limits().await;

		TestApiToken {
			id: response.id,
			token: response.token,
			name,
		}
	}

	/// Add a second user to a workspace with a specific role. Returns the
	/// second user's TestUser.
	pub async fn add_user_to_workspace_with_role(
		&self,
		admin_token: &BearerToken,
		workspace_id: Uuid,
		role_id: Uuid,
	) -> TestUser {
		let user_b = self.create_test_user().await;

		self.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id,
					user_id: user_b.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: admin_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest {
					roles: vec![role_id],
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			UpdateUserRolesInWorkspaceResponse,
		));

		self.clear_rate_limits().await;

		user_b
	}

	/// Create a managed URL in a workspace (redirect type).
	pub async fn create_test_managed_url(
		&self,
		token: &BearerToken,
		workspace_id: Uuid,
		domain_id: Uuid,
	) -> Uuid {
		let response = self
			.make_web_dashboard_call(
				ApiRequest::<CreateManagedURLRequest>::builder()
					.path(CreateManagedURLPath { workspace_id })
					.headers(CreateManagedURLRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateManagedURLRequest {
						sub_domain: random_name(6),
						domain_id,
						path: "/".to_string(),
						url_type: ManagedUrlType::Redirect {
							url: "https://example.com".to_string(),
							permanent_redirect: false,
							http_only: false,
						},
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
			.response;

		self.clear_rate_limits().await;

		response.id.id
	}
}
