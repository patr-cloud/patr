use std::collections::BTreeMap;

use models::{
	api::workspace::rbac::role::*,
	rbac::{DeploymentPermission, Permission, ResourcePermissionType},
};

use super::{all, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn rbac_view_roles_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;

	let response = setup
		.make_api_call(
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
		.make_api_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: ws_id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					name: random_name(8),
					description: "test".to_string(),
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
		.make_api_call(
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
		.make_api_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					name: random_name(8),
					description: "test".to_string(),
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
		.make_api_call(
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
		.make_api_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					name: random_name(8),
					description: "test".to_string(),
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
