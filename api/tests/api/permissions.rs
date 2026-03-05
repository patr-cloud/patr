use std::collections::{BTreeMap, BTreeSet};

use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::{
			*,
			container_registry::*,
			deployment::*,
			domain::*,
			managed_url::*,
			rbac::{role::*},
			runner::*,
			volume::*,
		},
	},
	rbac::ResourcePermissionType,
	utils::Uuid,
};

use crate::prelude::*;

// ---------------------------------------------------------------------------
// Helper: set up admin + workspace + specific permission for user B
// ---------------------------------------------------------------------------

/// Create admin, workspace, and user B with a role that has specific
/// permissions. Returns (admin, workspace_id, user_b).
async fn setup_permission_test(
	setup: &crate::api::setup::TestSetup,
	perm_entries: Vec<(&str, ResourcePermissionType)>,
) -> (TestUser, Uuid, TestUser) {
	let admin = create_test_user(setup).await;
	let ws = create_test_workspace(setup, &admin.access_token).await;

	let perm_ids =
		get_all_permission_ids(setup, &admin.access_token, ws.id).await;

	let mut permissions = BTreeMap::new();
	for (perm_name, perm_type) in perm_entries {
		let perm_id = perm_ids
			.get(perm_name)
			.unwrap_or_else(|| panic!("permission '{}' not found", perm_name));
		permissions.insert(*perm_id, perm_type);
	}

	let role = create_role_with_permissions(
		setup,
		&admin.access_token,
		ws.id,
		permissions,
	)
	.await;

	let user_b = add_user_to_workspace_with_role(
		setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	(admin, ws.id, user_b)
}

fn include(ids: &[Uuid]) -> ResourcePermissionType {
	ResourcePermissionType::Include(ids.iter().copied().collect())
}

fn exclude(ids: &[Uuid]) -> ResourcePermissionType {
	ResourcePermissionType::Exclude(ids.iter().copied().collect())
}

fn all() -> ResourcePermissionType {
	ResourcePermissionType::Exclude(BTreeSet::new())
}

// ===========================================================================
// 7a. Permission Grant Tests — Verify access IS granted
// ===========================================================================

#[tokio::test]
async fn deployment_view_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let view_id = perm_ids["deployment::view"];

	let mut perms = BTreeMap::new();
	perms.insert(view_id, include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with deployment::view should be able to get deployment info"
	);
}

#[tokio::test]
async fn deployment_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("deployment::create", all())],
	)
	.await;
	let runner = create_test_runner(&setup, &admin.access_token, ws_id).await;

	// Get machine type
	let mt = setup
		.server
		.method(
			ListAllDeploymentMachineTypeRequest::METHOD,
			&ListAllDeploymentMachineTypePath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&admin.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();
	let mt_id = mt.response.machine_types[0].id;

	let response = setup
		.server
		.method(
			CreateDeploymentRequest::METHOD,
			&CreateDeploymentPath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateDeploymentRequest {
			name: random_name(8),
			registry: DeploymentRegistry::ExternalRegistry {
				registry: "docker.io".to_string(),
				image_name: "library/nginx".to_string(),
			},
			image_tag: "latest".to_string(),
			runner: runner.id,
			machine_type: mt_id,
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
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn deployment_delete_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::delete"], include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			DeleteDeploymentRequest::METHOD,
			&DeleteDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn volume_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("volume::create", all())],
	)
	.await;

	let response = setup
		.server
		.method(
			CreateVolumeRequest::METHOD,
			&CreateVolumePath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateVolumeRequest {
			name: random_name(8),
			size: 1,
		})
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn runner_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("runner::create", all())],
	)
	.await;

	let response = setup
		.server
		.method(
			AddRunnerToWorkspaceRequest::METHOD,
			&AddRunnerToWorkspacePath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&AddRunnerToWorkspaceRequest {
			name: random_name(8),
		})
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn domain_add_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("domain::add", all())],
	)
	.await;

	let response = setup
		.server
		.method(
			AddDomainToWorkspaceRequest::METHOD,
			&AddDomainToWorkspacePath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&AddDomainToWorkspaceRequest {
			domain: format!("{}.com", random_name(8)),
			nameserver_type: DomainNameserverType::External,
		})
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn container_registry_create_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("containerRegistryRepository::create", all())],
	)
	.await;

	let response = setup
		.server
		.method(
			CreateContainerRepositoryRequest::METHOD,
			&CreateContainerRepositoryPath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateContainerRepositoryRequest {
			name: random_name(8),
		})
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn rbac_view_roles_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("viewRoles", all())],
	)
	.await;

	let response = setup
		.server
		.method(
			ListAllRolesRequest::METHOD,
			&ListAllRolesPath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn rbac_modify_roles_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("modifyRoles", all())],
	)
	.await;

	let response = setup
		.server
		.method(
			CreateNewRoleRequest::METHOD,
			&CreateNewRolePath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateNewRoleRequest {
			name: random_name(8),
			description: "test".to_string(),
			permissions: BTreeMap::new(),
		})
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn edit_workspace_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![("editWorkspace", all())],
	)
	.await;

	let response = setup
		.server
		.method(
			UpdateWorkspaceInfoRequest::METHOD,
			&UpdateWorkspaceInfoPath {
				workspace_id: ws_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&UpdateWorkspaceInfoRequest {
			name: Some(random_name(8)),
		})
		.await;

	assert!(response.status_code().is_success());
}

// ===========================================================================
// 7b. Permission Denial Tests — Verify access IS denied
// ===========================================================================

#[tokio::test]
async fn deployment_view_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	// Grant only deployment::create, NOT view
	let (_admin2, _ws_id, _user_b) = setup_permission_test(
		&setup,
		vec![],
	)
	.await;

	// _user_b is in a DIFFERENT workspace. Let's instead add them to admin's
	// workspace with no relevant permission.
	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(
		perm_ids["deployment::create"],
		all(),
	);
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b2 = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b2.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without deployment::view should be denied"
	);
}

#[tokio::test]
async fn deployment_create_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	// Grant only view, not create
	perms.insert(perm_ids["deployment::view"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let mt = setup
		.server
		.method(
			ListAllDeploymentMachineTypeRequest::METHOD,
			&ListAllDeploymentMachineTypePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&admin.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();
	let mt_id = mt.response.machine_types[0].id;

	let response = setup
		.server
		.method(
			CreateDeploymentRequest::METHOD,
			&CreateDeploymentPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateDeploymentRequest {
			name: random_name(8),
			registry: DeploymentRegistry::ExternalRegistry {
				registry: "docker.io".to_string(),
				image_name: "library/nginx".to_string(),
			},
			image_tag: "latest".to_string(),
			runner: runner.id,
			machine_type: mt_id,
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
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without deployment::create should be denied"
	);
}

#[tokio::test]
async fn volume_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let vol = create_test_volume(&setup, &admin.access_token, ws.id).await;

	// Give user B only viewRoles, no volume permissions
	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: vol.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without volume permissions should be denied"
	);
}

#[tokio::test]
async fn runner_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without runner::view should be denied"
	);
}

#[tokio::test]
async fn rbac_view_roles_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;

	// Give user B deployment::view only, not viewRoles
	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::view"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			ListAllRolesRequest::METHOD,
			&ListAllRolesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without viewRoles should be denied"
	);
}

#[tokio::test]
async fn rbac_modify_roles_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;

	// Give user B viewRoles only, not modifyRoles
	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			CreateNewRoleRequest::METHOD,
			&CreateNewRolePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateNewRoleRequest {
			name: random_name(8),
			description: "test".to_string(),
			permissions: BTreeMap::new(),
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without modifyRoles should be denied"
	);
}

#[tokio::test]
async fn edit_workspace_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			UpdateWorkspaceInfoRequest::METHOD,
			&UpdateWorkspaceInfoPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&UpdateWorkspaceInfoRequest {
			name: Some(random_name(8)),
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without editWorkspace should be denied"
	);
}

#[tokio::test]
async fn delete_workspace_denied_non_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["editWorkspace"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			DeleteWorkspaceRequest::METHOD,
			&DeleteWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-super-admin should not be able to delete workspace"
	);
}

// ===========================================================================
// 7c. Include List Tests — Verify scoped access
// ===========================================================================

#[tokio::test]
async fn deployment_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep1 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;
	let dep2 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::view"], include(&[dep1.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// dep1 — should succeed
	let r1 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "dep1 should be accessible");

	// dep2 — should fail
	let r2 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"dep2 should NOT be accessible"
	);
}

#[tokio::test]
async fn volume_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let vol1 = create_test_volume(&setup, &admin.access_token, ws.id).await;
	let vol2 = create_test_volume(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	// GetVolumeInfo uses Volume::Delete permission
	perms.insert(perm_ids["volume::delete"], include(&[vol1.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: vol1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "vol1 should be accessible");

	let r2 = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: vol2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"vol2 should NOT be accessible"
	);
}

#[tokio::test]
async fn runner_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner1 = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let runner2 = create_test_runner(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["runner::view"], include(&[runner1.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success());

	let r2 = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r2.status_code().is_client_error());
}

// ===========================================================================
// 7d. Exclude List Tests — Verify broad access with exceptions
// ===========================================================================

#[tokio::test]
async fn deployment_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep1 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;
	let dep2 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;
	let dep3 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	// Exclude dep2 — should have access to dep1 and dep3 but not dep2
	perms.insert(perm_ids["deployment::view"], exclude(&[dep2.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "dep1 should be accessible");

	let r2 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"dep2 should be excluded"
	);

	let r3 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep3.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r3.status_code().is_success(), "dep3 should be accessible");
}

#[tokio::test]
async fn deployment_exclude_empty_grants_all() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep1 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;
	let dep2 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::view"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success());

	let r2 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r2.status_code().is_success());
}

// ===========================================================================
// 7e. Cross-Permission Tests
// ===========================================================================

#[tokio::test]
async fn deployment_view_does_not_grant_edit() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::view"], include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// View should succeed
	let r_view = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r_view.status_code().is_success());

	// Edit should fail
	let r_edit = setup
		.server
		.method(
			UpdateDeploymentRequest::METHOD,
			&UpdateDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&UpdateDeploymentRequest {
			name: Some(random_name(8)),
			runner: None,
			machine_type: None,
			deploy_on_push: None,
			min_horizontal_scale: None,
			max_horizontal_scale: None,
			ports: None,
			environment_variables: None,
			startup_probe: None,
			liveness_probe: None,
			config_mounts: None,
			volumes: None,
		})
		.await;
	assert!(
		r_edit.status_code().is_client_error(),
		"view permission should not grant edit"
	);
}

#[tokio::test]
async fn deployment_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::view"], include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			DeleteDeploymentRequest::METHOD,
			&DeleteDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"view permission should not grant delete"
	);
}

#[tokio::test]
async fn rbac_view_does_not_grant_modify() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// List roles should succeed
	let r_list = setup
		.server
		.method(
			ListAllRolesRequest::METHOD,
			&ListAllRolesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r_list.status_code().is_success());

	// Create role should fail
	let r_create = setup
		.server
		.method(
			CreateNewRoleRequest::METHOD,
			&CreateNewRolePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateNewRoleRequest {
			name: random_name(8),
			description: "test".to_string(),
			permissions: BTreeMap::new(),
		})
		.await;
	assert!(
		r_create.status_code().is_client_error(),
		"viewRoles should not grant modifyRoles"
	);
}

// ===========================================================================
// 7f. Workspace Membership Tests — List endpoints
// ===========================================================================

#[tokio::test]
async fn list_deployments_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let non_member = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ListDeploymentRequest::METHOD,
			&ListDeploymentPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&non_member.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-member should not be able to list deployments"
	);
}

#[tokio::test]
async fn list_volumes_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let non_member = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ListVolumesInWorkspaceRequest::METHOD,
			&ListVolumesInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&non_member.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_runners_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let non_member = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ListRunnersForWorkspaceRequest::METHOD,
			&ListRunnersForWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&non_member.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_domains_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let non_member = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ListDomainsInWorkspaceRequest::METHOD,
			&ListDomainsInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&non_member.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_repositories_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let non_member = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ListContainerRepositoriesRequest::METHOD,
			&ListContainerRepositoriesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&non_member.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_managed_urls_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let non_member = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			ListManagedURLRequest::METHOD,
			&ListManagedURLPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&non_member.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_endpoints_allowed_for_any_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;

	// Add user B with minimal permissions (viewRoles only)
	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// All list endpoints use WorkspaceMembershipAuthenticator
	let list_deployments = setup
		.server
		.method(
			ListDeploymentRequest::METHOD,
			&ListDeploymentPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		list_deployments.status_code().is_success(),
		"member should list deployments"
	);

	let list_runners = setup
		.server
		.method(
			ListRunnersForWorkspaceRequest::METHOD,
			&ListRunnersForWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		list_runners.status_code().is_success(),
		"member should list runners"
	);

	let list_volumes = setup
		.server
		.method(
			ListVolumesInWorkspaceRequest::METHOD,
			&ListVolumesInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		list_volumes.status_code().is_success(),
		"member should list volumes"
	);

	let list_domains = setup
		.server
		.method(
			ListDomainsInWorkspaceRequest::METHOD,
			&ListDomainsInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		list_domains.status_code().is_success(),
		"member should list domains"
	);

	let list_repos = setup
		.server
		.method(
			ListContainerRepositoriesRequest::METHOD,
			&ListContainerRepositoriesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		list_repos.status_code().is_success(),
		"member should list repositories"
	);

	let list_urls = setup
		.server
		.method(
			ListManagedURLRequest::METHOD,
			&ListManagedURLPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		list_urls.status_code().is_success(),
		"member should list managed URLs"
	);
}

// ===========================================================================
// Additional Grant Tests
// ===========================================================================

#[tokio::test]
async fn managed_url_add_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let domain = create_test_domain(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["managedUrl::add"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			CreateManagedURLRequest::METHOD,
			&CreateManagedURLPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&CreateManagedURLRequest {
			sub_domain: random_name(6),
			domain_id: domain.id,
			path: "/".to_string(),
			url_type: ManagedUrlType::Redirect {
				url: "https://example.com".to_string(),
				permanent_redirect: false,
				http_only: false,
			},
		})
		.await;

	assert!(
		response.status_code().is_success(),
		"user with managedUrl::add should create managed URL"
	);
}

#[tokio::test]
async fn domain_delete_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let domain = create_test_domain(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["domain::delete"], include(&[domain.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			DeleteDomainInWorkspaceRequest::METHOD,
			&DeleteDomainInWorkspacePath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with domain::delete should delete domain"
	);
}

#[tokio::test]
async fn domain_verify_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let domain = create_test_domain(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["domain::verify"], include(&[domain.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			VerifyDomainInWorkspaceRequest::METHOD,
			&VerifyDomainInWorkspacePath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	// May fail due to Cloudflare API, but should not be a 403
	assert!(
		!response.status_code().is_client_error()
			|| response.status_code().as_u16() != 403,
		"user with domain::verify should not get 403"
	);
}

#[tokio::test]
async fn container_registry_delete_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let repo =
		create_test_container_repo(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(
		perm_ids["containerRegistryRepository::delete"],
		include(&[repo.id]),
	);
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			DeleteContainerRepositoryRequest::METHOD,
			&DeleteContainerRepositoryPath {
				workspace_id: ws.id,
				repository_id: repo.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with containerRegistryRepository::delete should delete repo"
	);
}

#[tokio::test]
async fn managed_url_delete_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let domain = create_test_domain(&setup, &admin.access_token, ws.id).await;
	let url_id = create_test_managed_url(
		&setup,
		&admin.access_token,
		ws.id,
		domain.id,
	)
	.await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["managedUrl::delete"], include(&[url_id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			DeleteManagedURLRequest::METHOD,
			&DeleteManagedURLPath {
				workspace_id: ws.id,
				managed_url_id: url_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with managedUrl::delete should delete managed URL"
	);
}

// ===========================================================================
// Additional Denial Tests
// ===========================================================================

#[tokio::test]
async fn domain_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let domain = create_test_domain(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: domain.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without domain permissions should be denied"
	);
}

#[tokio::test]
async fn container_registry_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let repo =
		create_test_container_repo(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			GetContainerRepositoryInfoRequest::METHOD,
			&GetContainerRepositoryInfoPath {
				workspace_id: ws.id,
				repository_id: repo.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without container registry permissions should be denied"
	);
}

#[tokio::test]
async fn managed_url_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let domain = create_test_domain(&setup, &admin.access_token, ws.id).await;
	let url_id = create_test_managed_url(
		&setup,
		&admin.access_token,
		ws.id,
		domain.id,
	)
	.await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["viewRoles"], all());
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			UpdateManagedURLRequest::METHOD,
			&UpdateManagedURLPath {
				workspace_id: ws.id,
				managed_url_id: url_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.json(&UpdateManagedURLRequest {
			path: Some("/new".to_string()),
			url_type: None,
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without managedUrl permissions should be denied"
	);
}

#[tokio::test]
async fn deployment_stop_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	// Grant only view, not stop
	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::view"], include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let response = setup
		.server
		.method(
			StopDeploymentRequest::METHOD,
			&StopDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without deployment::stop should be denied"
	);
}

// ===========================================================================
// Additional Include/Exclude Scoping Tests
// ===========================================================================

#[tokio::test]
async fn domain_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let dom1 = create_test_domain(&setup, &admin.access_token, ws.id).await;
	let dom2 = create_test_domain(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["domain::view"], include(&[dom1.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: dom1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "dom1 should be accessible");

	let r2 = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: dom2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"dom2 should NOT be accessible"
	);
}

#[tokio::test]
async fn container_registry_delete_include_grants_only_listed_resource() {
	// NOTE: GetContainerRepositoryInfo checks permission against workspace_id,
	// not repository_id. DeleteContainerRepository checks against repository_id.
	// So we test include scoping on the delete endpoint instead.
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let repo1 =
		create_test_container_repo(&setup, &admin.access_token, ws.id).await;
	let repo2 =
		create_test_container_repo(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(
		perm_ids["containerRegistryRepository::delete"],
		include(&[repo1.id]),
	);
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// repo2 — should fail
	let r2 = setup
		.server
		.method(
			DeleteContainerRepositoryRequest::METHOD,
			&DeleteContainerRepositoryPath {
				workspace_id: ws.id,
				repository_id: repo2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"repo2 should NOT be deletable"
	);

	// repo1 — should succeed
	let r1 = setup
		.server
		.method(
			DeleteContainerRepositoryRequest::METHOD,
			&DeleteContainerRepositoryPath {
				workspace_id: ws.id,
				repository_id: repo1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "repo1 should be deletable");
}

#[tokio::test]
async fn volume_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let vol1 = create_test_volume(&setup, &admin.access_token, ws.id).await;
	let vol2 = create_test_volume(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	// Exclude vol2 — access to vol1 but not vol2
	perms.insert(perm_ids["volume::delete"], exclude(&[vol2.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: vol1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "vol1 should be accessible");

	let r2 = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: vol2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"vol2 should be excluded"
	);
}

#[tokio::test]
async fn runner_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner1 = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let runner2 = create_test_runner(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["runner::view"], exclude(&[runner2.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "runner1 should be accessible");

	let r2 = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"runner2 should be excluded"
	);
}

#[tokio::test]
async fn domain_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let dom1 = create_test_domain(&setup, &admin.access_token, ws.id).await;
	let dom2 = create_test_domain(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["domain::view"], exclude(&[dom2.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: dom1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "dom1 should be accessible");

	let r2 = setup
		.server
		.method(
			GetDomainInfoInWorkspaceRequest::METHOD,
			&GetDomainInfoInWorkspacePath {
				workspace_id: ws.id,
				domain_id: dom2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"dom2 should be excluded"
	);
}

#[tokio::test]
async fn deployment_include_multiple_resources() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep1 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;
	let dep2 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;
	let dep3 =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(
		perm_ids["deployment::view"],
		include(&[dep1.id, dep2.id]),
	);
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	let r1 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep1.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r1.status_code().is_success(), "dep1 should be accessible");

	let r2 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep2.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r2.status_code().is_success(), "dep2 should be accessible");

	let r3 = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep3.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r3.status_code().is_client_error(),
		"dep3 should NOT be accessible"
	);
}

// ===========================================================================
// Additional Cross-Permission Tests
// ===========================================================================

#[tokio::test]
async fn deployment_view_does_not_grant_start() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::view"], include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// View should succeed
	let r_view = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r_view.status_code().is_success());

	// Start should fail
	let r_start = setup
		.server
		.method(
			StartDeploymentRequest::METHOD,
			&StartDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r_start.status_code().is_client_error(),
		"view permission should not grant start"
	);
}

#[tokio::test]
async fn deployment_edit_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::edit"], include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// Delete should fail
	let response = setup
		.server
		.method(
			DeleteDeploymentRequest::METHOD,
			&DeleteDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"edit permission should not grant delete"
	);
}

#[tokio::test]
async fn deployment_start_does_not_grant_stop() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["deployment::start"], include(&[dep.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// Stop should fail
	let response = setup
		.server
		.method(
			StopDeploymentRequest::METHOD,
			&StopDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"start permission should not grant stop"
	);
}

#[tokio::test]
async fn runner_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;

	let perm_ids =
		get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let mut perms = BTreeMap::new();
	perms.insert(perm_ids["runner::view"], include(&[runner.id]));
	let role =
		create_role_with_permissions(&setup, &admin.access_token, ws.id, perms)
			.await;
	let user_b = add_user_to_workspace_with_role(
		&setup,
		&admin.access_token,
		ws.id,
		role.id,
	)
	.await;

	// View should succeed
	let r_view = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(r_view.status_code().is_success());

	// Delete should fail
	let r_delete = setup
		.server
		.method(
			DeleteRunnerRequest::METHOD,
			&DeleteRunnerPath {
				workspace_id: ws.id,
				runner_id: runner.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
		.await;
	assert!(
		r_delete.status_code().is_client_error(),
		"view permission should not grant delete"
	);
}
