use std::collections::BTreeMap;

use models::{
	api::workspace::{rbac::role::*, volume::*},
	rbac::{DeploymentPermission, Permission, ResourcePermissionType, VolumePermission},
};

use super::{all, include, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn rbac_view_roles_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllRolesRequest>::builder()
				.path(ListAllRolesPath {
					workspace_id: ws_id,
				})
				.headers(ListAllRolesRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn rbac_modify_roles_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ModifyRoles, all())]).await;

	// Role creation rejects empty permissions with `WrongParameters`, so seed
	// one harmless permission.
	let mut permissions = BTreeMap::new();
	permissions.insert(
		setup.get_permission_id(Permission::ViewRoles),
		ResourcePermissionType::Include(Default::default()),
	);

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: ws_id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test".to_string(),
					},
					permissions,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn rbac_view_roles_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Deployment(DeploymentPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllRolesRequest>::builder()
				.path(ListAllRolesPath {
					workspace_id: workspace.id,
				})
				.headers(ListAllRolesRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without viewRoles should be denied"
	);
}

#[tokio::test]
async fn rbac_modify_roles_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let mut perms = BTreeMap::new();
	perms.insert(setup.get_permission_id(Permission::ViewRoles), all());
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test".to_string(),
					},
					permissions: BTreeMap::new(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without modifyRoles should be denied"
	);
}

#[tokio::test]
async fn rbac_view_does_not_grant_modify() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let mut perms = BTreeMap::new();
	perms.insert(setup.get_permission_id(Permission::ViewRoles), all());
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// List roles should succeed
	let r_list = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllRolesRequest>::builder()
				.path(ListAllRolesPath {
					workspace_id: workspace.id,
				})
				.headers(ListAllRolesRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_list.status_code().is_success());

	// Create role should fail
	let r_create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test".to_string(),
					},
					permissions: BTreeMap::new(),
				})
				.build(),
		)
		.await;
	assert!(
		r_create.status_code().is_client_error(),
		"viewRoles should not grant modifyRoles"
	);
}

/// Regression test for the loader join bug where a role's include/exclude
/// resource lists were attached to EVERY permission of the role instead of
/// only the permission they belong to (the LEFT JOIN omitted
/// `permission_id`). A role granting `Volume::View` on v1 and
/// `Volume::Delete` on v2 must not let the user view v2 or delete v1.
///
/// Note: non-uniform roles like this become unrepresentable once role
/// bindings land (PR 10 of the stack) — this test will be reshaped then.
#[tokio::test]
async fn rbac_include_lists_do_not_cross_permissions() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let volume1 = setup
		.create_test_volume(&admin.access_token, workspace.id)
		.await;
	let volume2 = setup
		.create_test_volume(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Volume(VolumePermission::View)),
		include(&[volume1.id]),
	);
	perms.insert(
		setup.get_permission_id(Permission::Volume(VolumePermission::Delete)),
		include(&[volume2.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// View v1 is granted directly
	let r_view_v1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume1.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r_view_v1.status_code().is_success(),
		"view on volume1 is granted directly"
	);

	// Delete v1 must fail: v1 is only in the View include list
	let r_delete_v1 = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteVolumeRequest>::builder()
				.path(DeleteVolumePath {
					workspace_id: workspace.id,
					volume_id: volume1.id,
				})
				.headers(DeleteVolumeRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r_delete_v1.status_code().is_client_error(),
		"delete include list must not leak volume1 from the view include list"
	);

	// View v2 must fail: v2 is only in the Delete include list
	let r_view_v2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume2.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r_view_v2.status_code().is_client_error(),
		"view include list must not leak volume2 from the delete include list"
	);
}
