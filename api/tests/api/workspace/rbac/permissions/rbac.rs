use std::collections::BTreeSet;

use models::{
	api::workspace::rbac::role::*,
	rbac::{DeploymentPermission, Permission},
};

use super::{all, setup_permission_test};
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
	let permissions = vec![setup.get_permission_id(Permission::ViewRoles)];

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
					permissions: permissions.into_iter().collect(),
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

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Deployment(DeploymentPermission::View))],
		)
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

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
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
					permissions: BTreeSet::new(),
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

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
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
					permissions: BTreeSet::new(),
				})
				.build(),
		)
		.await;
	assert!(
		r_create.status_code().is_client_error(),
		"viewRoles should not grant modifyRoles"
	);
}
