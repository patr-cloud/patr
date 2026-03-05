use std::collections::BTreeMap;

use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		auth::*,
		user::*,
		workspace::{
			*,
			container_registry::*,
			deployment::*,
			domain::*,
			managed_url::*,
			rbac::{*, role::*, user::*},
			runner::*,
			volume::*,
		},
	},
	rbac::ResourcePermissionType,
	utils::Uuid,
};
use rand::{RngExt, distr::Alphanumeric};

use crate::api::setup::TestSetup;

/// A test user with credentials and tokens.
#[allow(missing_docs)]
pub struct TestUser {
	pub user_id: Uuid,
	pub username: String,
	pub password: String,
	pub access_token: String,
	pub refresh_token: String,
}

/// A test workspace.
#[allow(missing_docs)]
pub struct TestWorkspace {
	pub id: Uuid,
	pub name: String,
}

/// A test runner.
#[allow(missing_docs)]
pub struct TestRunner {
	pub id: Uuid,
	pub name: String,
}

/// A test deployment.
#[allow(missing_docs)]
pub struct TestDeployment {
	pub id: Uuid,
	pub name: String,
}

/// A test domain.
#[allow(missing_docs)]
pub struct TestDomain {
	pub id: Uuid,
	pub domain: String,
}

/// A test volume.
#[allow(missing_docs)]
pub struct TestVolume {
	pub id: Uuid,
	pub name: String,
}

/// A test container repository.
#[allow(missing_docs)]
pub struct TestContainerRepo {
	pub id: Uuid,
	pub name: String,
}

/// A test role.
#[allow(missing_docs)]
pub struct TestRole {
	pub id: Uuid,
	pub name: String,
}

/// A test API token.
#[allow(missing_docs)]
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
			.sample_iter(Alphanumeric)
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
			.sample_iter(Alphanumeric)
			.map(char::from)
			.take(28)
			.collect::<String>()
	)
}

/// Create a new test user account (CreateAccount + CompleteSignUp), returning
/// the user's credentials and tokens.
pub async fn create_test_user(setup: &TestSetup) -> TestUser {
	let username = random_name(8);
	let password = random_password();

	setup
		.server
		.method(
			CreateAccountRequest::METHOD,
			&CreateAccountPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.json(&CreateAccountRequest {
			username: username.clone(),
			password: password.clone(),
			first_name: "Test".to_string(),
			last_name: "User".to_string(),
			recovery_method: RecoveryMethod::Email {
				recovery_email: format!("{}@example.com", &username),
			},
			cf_turnstile_token: "1x00000000000000000000AA".to_string(),
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	let response = setup
		.server
		.method(
			CompleteSignUpRequest::METHOD,
			&CompleteSignUpPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.json(&CompleteSignUpRequest {
			username: username.clone(),
			verification_token: "000000".to_string(),
			cf_turnstile_token: "1x00000000000000000000AA".to_string(),
		})
		.await
		.json::<ApiSuccessResponseBody<CompleteSignUpResponse>>()
		.response;

	let user_info = setup
		.server
		.method(GetUserInfoRequest::METHOD, &GetUserInfoPath.to_string())
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&response.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	TestUser {
		user_id: user_info.response.basic_user_info.id,
		username,
		password,
		access_token: response.access_token,
		refresh_token: response.refresh_token,
	}
}

/// Login an existing test user, returning new access and refresh tokens.
pub async fn login_test_user(
	setup: &TestSetup,
	username: &str,
	password: &str,
) -> (String, String) {
	let response = setup
		.server
		.method(LoginRequest::METHOD, &LoginPath.to_string())
		.add_header(header::USER_AGENT, "cargo-test")
		.json(&LoginRequest {
			user_id: username.to_string(),
			password: password.to_string(),
			mfa_otp: None,
			cf_turnstile_token: "1x00000000000000000000AA".to_string(),
		})
		.await
		.json::<ApiSuccessResponseBody<LoginResponse>>()
		.response;

	(response.access_token, response.refresh_token)
}

/// Create a new workspace, returning its ID and name.
pub async fn create_test_workspace(setup: &TestSetup, token: &str) -> TestWorkspace {
	let name = random_name(8);

	let response = setup
		.server
		.method(
			CreateWorkspaceRequest::METHOD,
			&CreateWorkspacePath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateWorkspaceRequest { name: name.clone() })
		.await
		.json::<ApiSuccessResponseBody<CreateWorkspaceResponse>>()
		.response;

	TestWorkspace {
		id: response.id.id,
		name,
	}
}

/// Add a runner to a workspace, returning its ID and name.
pub async fn create_test_runner(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
) -> TestRunner {
	let name = random_name(8);

	let response = setup
		.server
		.method(
			AddRunnerToWorkspaceRequest::METHOD,
			&AddRunnerToWorkspacePath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&AddRunnerToWorkspaceRequest { name: name.clone() })
		.await
		.json::<ApiSuccessResponseBody<AddRunnerToWorkspaceResponse>>()
		.response;

	TestRunner {
		id: response.id.id,
		name,
	}
}

/// Create a deployment using an external image (nginx), returning its ID and
/// name.
pub async fn create_test_deployment(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
	runner_id: Uuid,
) -> TestDeployment {
	let name = random_name(8);

	// First get a valid machine type
	let machine_types = setup
		.server
		.method(
			ListAllDeploymentMachineTypeRequest::METHOD,
			&ListAllDeploymentMachineTypePath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();

	let machine_type_id = machine_types
		.response
		.machine_types
		.first()
		.expect("no machine types available")
		.id;

	let response = setup
		.server
		.method(
			CreateDeploymentRequest::METHOD,
			&CreateDeploymentPath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateDeploymentRequest {
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
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	TestDeployment {
		id: response.id.id,
		name,
	}
}

/// Add a domain to a workspace, returning its ID and domain name.
pub async fn create_test_domain(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
) -> TestDomain {
	let domain = format!("{}.com", random_name(8));

	let response = setup
		.server
		.method(
			AddDomainToWorkspaceRequest::METHOD,
			&AddDomainToWorkspacePath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&AddDomainToWorkspaceRequest {
			domain: domain.clone(),
			nameserver_type: DomainNameserverType::External,
		})
		.await
		.json::<ApiSuccessResponseBody<AddDomainToWorkspaceResponse>>()
		.response;

	TestDomain {
		id: response.id.id,
		domain,
	}
}

/// Create a volume in a workspace, returning its ID and name.
pub async fn create_test_volume(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
) -> TestVolume {
	let name = random_name(8);

	let response = setup
		.server
		.method(
			CreateVolumeRequest::METHOD,
			&CreateVolumePath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateVolumeRequest {
			name: name.clone(),
			size: 1,
		})
		.await
		.json::<ApiSuccessResponseBody<CreateVolumeResponse>>()
		.response;

	TestVolume {
		id: response.id.id,
		name,
	}
}

/// Create a container repository in a workspace, returning its ID and name.
pub async fn create_test_container_repo(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
) -> TestContainerRepo {
	let name = random_name(8);

	let response = setup
		.server
		.method(
			CreateContainerRepositoryRequest::METHOD,
			&CreateContainerRepositoryPath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateContainerRepositoryRequest { name: name.clone() })
		.await
		.json::<ApiSuccessResponseBody<CreateContainerRepositoryResponse>>()
		.response;

	TestContainerRepo {
		id: response.id.id,
		name,
	}
}

/// Create a role in a workspace with the given permissions, returning its ID
/// and name.
pub async fn create_test_role(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
) -> TestRole {
	let name = random_name(8);

	let response = setup
		.server
		.method(
			CreateNewRoleRequest::METHOD,
			&CreateNewRolePath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateNewRoleRequest {
			name: name.clone(),
			description: "test role".to_string(),
			permissions: BTreeMap::new(),
		})
		.await
		.json::<ApiSuccessResponseBody<CreateNewRoleResponse>>()
		.response;

	TestRole {
		id: response.id.id,
		name,
	}
}

/// Create a role with specific permissions.
pub async fn create_role_with_permissions(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
	permissions: BTreeMap<Uuid, ResourcePermissionType>,
) -> TestRole {
	let name = random_name(8);

	let response = setup
		.server
		.method(
			CreateNewRoleRequest::METHOD,
			&CreateNewRolePath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateNewRoleRequest {
			name: name.clone(),
			description: "test role with permissions".to_string(),
			permissions,
		})
		.await
		.json::<ApiSuccessResponseBody<CreateNewRoleResponse>>()
		.response;

	TestRole {
		id: response.id.id,
		name,
	}
}

/// Create an API token for the user, returning its ID, token string, and name.
pub async fn create_test_api_token(setup: &TestSetup, token: &str) -> TestApiToken {
	let name = random_name(8);

	let response = setup
		.server
		.method(
			CreateApiTokenRequest::METHOD,
			&CreateApiTokenPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateApiTokenRequest {
			token: UserApiToken {
				name: name.clone(),
				permissions: BTreeMap::new(),
				token_nbf: None,
				token_exp: None,
				allowed_ips: None,
				created: time::OffsetDateTime::now_utc(),
			},
		})
		.await
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response;

	TestApiToken {
		id: response.id,
		token: response.token,
		name,
	}
}

/// Add a second user to a workspace with a specific role. Returns the second
/// user's TestUser.
pub async fn add_user_to_workspace_with_role(
	setup: &TestSetup,
	admin_token: &str,
	workspace_id: Uuid,
	role_id: Uuid,
) -> TestUser {
	let user_b = create_test_user(setup).await;

	setup
		.server
		.method(
			UpdateUserRolesInWorkspaceRequest::METHOD,
			&UpdateUserRolesInWorkspacePath {
				workspace_id,
				user_id: user_b.user_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(admin_token)
		.json(&UpdateUserRolesInWorkspaceRequest {
			roles: vec![role_id],
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			UpdateUserRolesInWorkspaceResponse,
		));

	user_b
}

/// Look up the UUID of a permission by its string name (e.g.
/// "deployment::create") from ListAllPermissions.
pub async fn get_permission_id(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
	permission_name: &str,
) -> Uuid {
	let perms = setup
		.server
		.method(
			ListAllPermissionsRequest::METHOD,
			&ListAllPermissionsPath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.await
		.json::<ApiSuccessResponseBody<ListAllPermissionsResponse>>();

	perms
		.response
		.permissions
		.iter()
		.find(|p| p.data.name == permission_name)
		.unwrap_or_else(|| panic!("permission '{}' not found", permission_name))
		.id
}

/// Get all permission name→UUID mappings from ListAllPermissions.
pub async fn get_all_permission_ids(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
) -> BTreeMap<String, Uuid> {
	let perms = setup
		.server
		.method(
			ListAllPermissionsRequest::METHOD,
			&ListAllPermissionsPath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.await
		.json::<ApiSuccessResponseBody<ListAllPermissionsResponse>>();

	perms
		.response
		.permissions
		.into_iter()
		.map(|p| (p.data.name, p.id))
		.collect()
}

/// Create a managed URL in a workspace (redirect type).
pub async fn create_test_managed_url(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
	domain_id: Uuid,
) -> Uuid {
	let response = setup
		.server
		.method(
			CreateManagedURLRequest::METHOD,
			&CreateManagedURLPath { workspace_id }.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(token)
		.json(&CreateManagedURLRequest {
			sub_domain: random_name(6),
			domain_id,
			path: "/".to_string(),
			url_type: ManagedUrlType::Redirect {
				url: "https://example.com".to_string(),
				permanent_redirect: false,
				http_only: false,
			},
		})
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
		.response;

	response.id.id
}
