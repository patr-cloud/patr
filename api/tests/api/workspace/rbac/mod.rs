use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::rbac::{role::*, user::*, *},
	rbac::{Permission, ResourcePermissionType},
};

use crate::prelude::*;

mod permissions;

#[tokio::test]
async fn list_all_permissions_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListAllPermissionsRequest>::builder()
				.path(ListAllPermissionsPath {
					workspace_id: workspace.id,
				})
				.headers(ListAllPermissionsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllPermissionsResponse>>();

	assert!(
		!response.response.permissions.is_empty(),
		"permissions list should not be empty"
	);
}

#[tokio::test]
async fn list_all_resource_types_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListAllResourceTypesRequest>::builder()
				.path(ListAllResourceTypesPath {
					workspace_id: workspace.id,
				})
				.headers(ListAllResourceTypesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllResourceTypesResponse>>();

	assert!(
		!response.response.resource_types.is_empty(),
		"resource types list should not be empty"
	);
}

#[tokio::test]
async fn get_current_permissions_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetCurrentPermissionsRequest>::builder()
				.path(GetCurrentPermissionsPath {
					workspace_id: workspace.id,
				})
				.headers(GetCurrentPermissionsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetCurrentPermissionsResponse>>();

	assert!(
		response.response.permissions.is_super_admin(),
		"workspace creator should be super admin"
	);
}

#[tokio::test]
async fn create_role_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;
	assert!(!role.name.is_empty());
}

#[tokio::test]
async fn list_roles_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListAllRolesRequest>::builder()
				.path(ListAllRolesPath {
					workspace_id: workspace.id,
				})
				.headers(ListAllRolesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllRolesResponse>>();

	assert!(!response.response.roles.is_empty());
}

#[tokio::test]
async fn get_role_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetRoleInfoRequest>::builder()
				.path(GetRoleInfoPath {
					workspace_id: workspace.id,
					role_id: role.id,
				})
				.headers(GetRoleInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetRoleInfoResponse>>();

	assert_eq!(role.name, response.response.role.name);
}

#[tokio::test]
async fn update_role_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;
	let new_name = random_name(8);

	setup
		.make_api_call(
			ApiRequest::<UpdateRoleRequest>::builder()
				.path(UpdateRolePath {
					workspace_id: workspace.id,
					role_id: role.id,
				})
				.headers(UpdateRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateRoleRequest {
					name: Some(new_name.clone()),
					description: None,
					permissions: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateRoleResponse));
}

#[tokio::test]
async fn delete_role_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	setup
		.make_api_call(
			ApiRequest::<DeleteRoleRequest>::builder()
				.path(DeleteRolePath {
					workspace_id: workspace.id,
					role_id: role.id,
				})
				.headers(DeleteRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(DeleteRoleQuery {
					remove_users: false,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRoleResponse));
}

#[tokio::test]
async fn list_users_for_role_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListUsersForRoleRequest>::builder()
				.path(ListUsersForRolePath {
					workspace_id: workspace.id,
					role_id: role.id,
				})
				.headers(ListUsersForRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUsersForRoleResponse>>();

	// New role, no users assigned yet
	assert!(response.response.users.is_empty());
}

#[tokio::test]
async fn list_users_in_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListUsersInWorkspaceRequest>::builder()
				.path(ListUsersInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListUsersInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUsersInWorkspaceResponse>>();

	// Super admin is not in workspace_user table, so creator won't appear here
	// unless explicitly added via UpdateUserRolesInWorkspace
	assert!(
		response.response.users.is_empty(),
		"workspace_user table should be empty for a new workspace"
	);
}

#[tokio::test]
async fn update_user_roles_works() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// Verify user B is in the workspace
	let response = setup
		.make_api_call(
			ApiRequest::<ListUsersInWorkspaceRequest>::builder()
				.path(ListUsersInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListUsersInWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUsersInWorkspaceResponse>>();

	assert!(response.response.users.contains_key(&user_b.user_id));
}

#[tokio::test]
async fn remove_user_from_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	setup
		.make_api_call(
			ApiRequest::<RemoveUserFromWorkspaceRequest>::builder()
				.path(RemoveUserFromWorkspacePath {
					workspace_id: workspace.id,
					user_id: user_b.user_id,
				})
				.headers(RemoveUserFromWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			RemoveUserFromWorkspaceResponse,
		));

	// Verify user B is gone
	let response = setup
		.make_api_call(
			ApiRequest::<ListUsersInWorkspaceRequest>::builder()
				.path(ListUsersInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListUsersInWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUsersInWorkspaceResponse>>();

	assert!(!response.response.users.contains_key(&user_b.user_id));
}

#[tokio::test]
async fn get_current_permissions_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let view_roles_id = setup.get_permission_id(Permission::ViewRoles);

	let mut permissions = BTreeMap::new();
	permissions.insert(
		view_roles_id,
		ResourcePermissionType::Exclude(std::collections::BTreeSet::new()),
	);

	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, permissions)
		.await;

	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetCurrentPermissionsRequest>::builder()
				.path(GetCurrentPermissionsPath {
					workspace_id: workspace.id,
				})
				.headers(GetCurrentPermissionsRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetCurrentPermissionsResponse>>();

	assert!(
		response.response.permissions.is_member(),
		"user B should be a member, not super admin"
	);
}

#[tokio::test]
async fn rbac_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListAllRolesRequest>::builder()
				.path(ListAllRolesPath {
					workspace_id: workspace.id,
				})
				.headers(ListAllRolesRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}
