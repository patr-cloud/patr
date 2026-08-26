use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::rbac::{role::*, user::*, *},
	rbac::{Permission, ResourcePermissionType},
};

use crate::prelude::*;

pub mod invite;
pub mod permissions;

#[tokio::test]
async fn list_all_permissions_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
					role: Role {
						name: new_name.clone(),
						description: "test role".to_string(),
					},
					permissions: BTreeMap::from([(
						setup.get_permission_id(Permission::ViewRoles),
						ResourcePermissionType::Include(Default::default()),
					)]),
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
async fn list_users_for_role_filters_by_role() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role_a = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let role_b = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;

	let user_a = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role_a.id)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role_b.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListUsersForRoleRequest>::builder()
				.path(ListUsersForRolePath {
					workspace_id: workspace.id,
					role_id: role_a.id,
				})
				.headers(ListUsersForRoleRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUsersForRoleResponse>>();

	// Must return only role_a's user, not role_b's — the query has to filter
	// by role_id, not just workspace_id.
	let user_ids = response
		.response
		.users
		.iter()
		.map(|u| u.id)
		.collect::<Vec<_>>();
	assert_eq!(user_ids, vec![user_a.user_id]);
	assert!(!user_ids.contains(&user_b.user_id));
}

#[tokio::test]
async fn list_users_in_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
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

	// The owner holds super-admin rights on the workspace directly rather than
	// through a role, so they have no `workspace_user` rows — but they're a
	// member all the same, and the endpoint UNIONs them in so the UI doesn't
	// have to synthesise the row itself.
	assert_eq!(response.response.users.len(), 1);
	let owner = &response.response.users[0];
	assert_eq!(owner.user.id, user.user_id);
	assert_eq!(owner.user.email, user.email);
	assert!(owner.is_owner, "the creator must be flagged as the owner");
	assert!(
		owner.role_ids.is_empty(),
		"the owner's access doesn't come from a role"
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
		.make_web_dashboard_call(
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

	assert!(
		response
			.response
			.users
			.iter()
			.any(|u| u.user.id == user_b.user_id)
	);
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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

	assert!(
		!response
			.response
			.users
			.iter()
			.any(|u| u.user.id == user_b.user_id)
	);
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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

#[tokio::test]
async fn create_role_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: role.name.clone(),
						description: "duplicate".to_string(),
					},
					permissions: BTreeMap::new(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate role name, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn delete_role_in_use() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let _user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRoleRequest>::builder()
				.path(DeleteRolePath {
					workspace_id: workspace.id,
					role_id: role.id,
				})
				.headers(DeleteRoleRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(DeleteRoleQuery {
					remove_users: false,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected RoleInUse for role still assigned to a user, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn delete_role_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRoleRequest>::builder()
				.path(DeleteRolePath {
					workspace_id: workspace.id,
					role_id: Uuid::nil(),
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
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected RoleDoesNotExist for missing role, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn update_role_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateRoleRequest>::builder()
				.path(UpdateRolePath {
					workspace_id: workspace.id,
					role_id: Uuid::nil(),
				})
				.headers(UpdateRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test role".to_string(),
					},
					permissions: BTreeMap::from([(
						setup.get_permission_id(Permission::ViewRoles),
						ResourcePermissionType::Include(Default::default()),
					)]),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected RoleDoesNotExist for missing role, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn update_role_add_permissions() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ViewRoles),
		ResourcePermissionType::Exclude(Default::default()),
	);

	setup
		.make_web_dashboard_call(
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
					role: Role {
						name: random_name(8),
						description: "test role".to_string(),
					},
					permissions: perms,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateRoleResponse));

	// Read back; the role should now have one permission entry.
	let response = setup
		.make_web_dashboard_call(
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

	assert!(
		!response.response.permissions.is_empty(),
		"role should have at least one permission after update"
	);
}

#[tokio::test]
async fn update_role_remove_permissions() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	// Start with two permissions.
	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ViewRoles),
		ResourcePermissionType::Exclude(Default::default()),
	);
	perms.insert(
		setup.get_permission_id(Permission::ModifyRoles),
		ResourcePermissionType::Exclude(Default::default()),
	);
	let role = setup
		.create_role_with_permissions(&user.access_token, workspace.id, perms)
		.await;

	// Replace with just one permission.
	let mut next = BTreeMap::new();
	next.insert(
		setup.get_permission_id(Permission::ViewRoles),
		ResourcePermissionType::Exclude(Default::default()),
	);

	setup
		.make_web_dashboard_call(
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
					role: Role {
						name: random_name(8),
						description: "test role".to_string(),
					},
					permissions: next,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateRoleResponse));

	let response = setup
		.make_web_dashboard_call(
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

	assert_eq!(
		response.response.permissions.len(),
		1,
		"role should have exactly one permission after removing one"
	);
}

#[tokio::test]
async fn update_user_roles_nonexistent_user() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: workspace.id,
					user_id: Uuid::nil(),
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest {
					roles: vec![role.id],
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected UserNotFound for nonexistent user, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn update_user_roles_nonexistent_role() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let user_b = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: workspace.id,
					user_id: user_b.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest {
					roles: vec![Uuid::nil()],
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected RoleDoesNotExist for nonexistent role, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn create_role_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ViewRoles),
		ResourcePermissionType::Include(Default::default()),
	);

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: "!!!".to_string(),
						description: "test".to_string(),
					},
					permissions: perms,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"role name failing RESOURCE_NAME_REGEX should be rejected"
	);
}

#[tokio::test]
async fn update_user_roles_idempotent() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// Call update_user_roles a second time with the same role — handler should
	// treat membership idempotently (replaces roles, no error).
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: workspace.id,
					user_id: user_b.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest {
					roles: vec![role.id],
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			UpdateUserRolesInWorkspaceResponse,
		));
}

#[tokio::test]
async fn update_user_roles_empty_keeps_membership() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// An empty roles list drops the user's bindings but keeps them a member
	// — removal is RemoveUserFromWorkspace's job.
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: workspace.id,
					user_id: user_b.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest { roles: vec![] })
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			UpdateUserRolesInWorkspaceResponse,
		));

	let is_member: bool = sqlx::query_scalar(&format!(
		"SELECT EXISTS(SELECT 1 FROM workspace_user WHERE user_id = '{}' AND workspace_id = '{}')",
		user_b.user_id, workspace.id
	))
	.fetch_one(setup.database())
	.await
	.expect("membership query");
	assert!(is_member, "empty roles must keep the membership row");

	let bindings: i64 = sqlx::query_scalar(&format!(
		"SELECT COUNT(*) FROM role_binding rb JOIN workspace_actor a ON a.id = rb.actor_id \
		 WHERE a.user_id = '{}' AND rb.workspace_id = '{}'",
		user_b.user_id, workspace.id
	))
	.fetch_one(setup.database())
	.await
	.expect("binding query");
	assert_eq!(0, bindings, "empty roles must drop every binding");
}

#[tokio::test]
async fn delete_role_soft_deletes_resource_row() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRoleRequest>::builder()
				.path(DeleteRolePath {
					workspace_id: workspace.id,
					role_id: role.id,
				})
				.query(DeleteRoleQuery {
					remove_users: false,
				})
				.headers(DeleteRoleRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(DeleteRoleRequest)
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRoleResponse));

	// The role's resource row is tombstoned like every other deleted
	// resource, not leaked live.
	let deleted: bool = sqlx::query_scalar(&format!(
		"SELECT deleted IS NOT NULL FROM resource WHERE id = '{}'",
		role.id
	))
	.fetch_one(setup.database())
	.await
	.expect("resource query");
	assert!(deleted, "deleted role must tombstone its resource row");
}

#[tokio::test]
async fn remove_user_from_workspace_not_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RemoveUserFromWorkspaceRequest>::builder()
				.path(RemoveUserFromWorkspacePath {
					workspace_id: workspace.id,
					user_id: Uuid::nil(),
				})
				.headers(RemoveUserFromWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected UserNotFound for non-member removal, got {}",
		response.status_code()
	);
}

/// Helper: attempt CreateNewRole with a given `description`, return the
/// response.
async fn create_role_with_description(
	setup: &TestSetup,
	user: &TestUser,
	workspace_id: Uuid,
	description: &str,
) -> ::axum_test::TestResponse {
	let mut permissions = BTreeMap::new();
	permissions.insert(
		setup.get_permission_id(Permission::ViewRoles),
		ResourcePermissionType::Include(Default::default()),
	);
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath { workspace_id })
				.headers(CreateNewRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: description.to_string(),
					},
					permissions,
				})
				.build(),
		)
		.await
}

#[tokio::test]
async fn create_role_rejects_xss_in_description() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response =
		create_role_with_description(&setup, &user, workspace.id, "<script>alert(1)</script>")
			.await;
	assert!(
		response.status_code().is_client_error(),
		"expected 4xx for HTML in description, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn create_role_rejects_over_500_char_description() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response =
		create_role_with_description(&setup, &user, workspace.id, &"a".repeat(501)).await;
	assert!(
		response.status_code().is_client_error(),
		"expected 4xx for over-length description, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn create_role_substitutes_default_text_for_empty_description() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	// Empty description should NOT 4xx — the handler substitutes a default.
	let response = create_role_with_description(&setup, &user, workspace.id, "").await;
	assert!(
		response.status_code().is_success(),
		"empty description should be accepted (default substituted), got {}",
		response.status_code()
	);

	// Fetch the role and confirm the default was stored.
	let role_id = response
		.json::<ApiSuccessResponseBody<CreateNewRoleResponse>>()
		.response
		.id
		.id;

	let role = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRoleInfoRequest>::builder()
				.path(GetRoleInfoPath {
					workspace_id: workspace.id,
					role_id,
				})
				.headers(GetRoleInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetRoleInfoResponse>>();

	assert_eq!("No description provided", role.response.role.description);
}

#[tokio::test]
async fn update_role_rejects_xss_in_description() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
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
					role: Role {
						name: random_name(8),
						description: "<script>alert(1)</script>".to_string(),
					},
					permissions: BTreeMap::from([(
						setup.get_permission_id(Permission::ViewRoles),
						ResourcePermissionType::Include(Default::default()),
					)]),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"expected 4xx for HTML in updated description, got {}",
		response.status_code()
	);
}

/// A role in workspace A cannot be read via workspace B's URL by B's owner.
#[tokio::test]
async fn role_cross_workspace_get_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let owner_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&owner_a.access_token).await;
	let role_a = setup.create_test_role(&owner_a.access_token, ws_a.id).await;

	let owner_b = setup.create_test_user().await;
	let ws_b = setup.create_test_workspace(&owner_b.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRoleInfoRequest>::builder()
				.path(GetRoleInfoPath {
					workspace_id: ws_b.id,
					role_id: role_a.id,
				})
				.headers(GetRoleInfoRequestHeaders {
					authorization: owner_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"reading workspace A's role via workspace B's URL should be denied"
	);
}

/// A role in workspace A cannot be updated via workspace B's URL by B's owner.
#[tokio::test]
async fn role_cross_workspace_update_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let owner_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&owner_a.access_token).await;
	let role_a = setup.create_test_role(&owner_a.access_token, ws_a.id).await;

	let owner_b = setup.create_test_user().await;
	let ws_b = setup.create_test_workspace(&owner_b.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateRoleRequest>::builder()
				.path(UpdateRolePath {
					workspace_id: ws_b.id,
					role_id: role_a.id,
				})
				.headers(UpdateRoleRequestHeaders {
					authorization: owner_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test role".to_string(),
					},
					permissions: BTreeMap::from([(
						setup.get_permission_id(Permission::ViewRoles),
						ResourcePermissionType::Include(Default::default()),
					)]),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"updating workspace A's role via workspace B's URL should be denied"
	);
}

/// A role in workspace A cannot be deleted via workspace B's URL by B's owner.
#[tokio::test]
async fn role_cross_workspace_delete_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let owner_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&owner_a.access_token).await;
	let role_a = setup.create_test_role(&owner_a.access_token, ws_a.id).await;

	let owner_b = setup.create_test_user().await;
	let ws_b = setup.create_test_workspace(&owner_b.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRoleRequest>::builder()
				.path(DeleteRolePath {
					workspace_id: ws_b.id,
					role_id: role_a.id,
				})
				.query(DeleteRoleQuery {
					remove_users: false,
				})
				.headers(DeleteRoleRequestHeaders {
					authorization: owner_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"deleting workspace A's role via workspace B's URL should be denied"
	);
}

/// A user cannot add a member to a workspace they don't own/administer.
#[tokio::test]
async fn add_member_to_unowned_workspace_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let owner_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&owner_a.access_token).await;
	let owner_b = setup.create_test_user().await;
	let _ws_b = setup.create_test_workspace(&owner_b.access_token).await;
	let outsider = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: ws_a.id,
					user_id: outsider.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: owner_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest { roles: vec![] })
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"adding a member to a workspace you don't own should be denied"
	);
}

/// Creating a workspace seeds the default set of 27 roles (workspace_id = the
/// workspace id).
#[tokio::test]
async fn default_roles_seeded_on_workspace_create() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let count: i64 = sqlx::query_scalar(&format!(
		"SELECT COUNT(*) FROM role WHERE workspace_id = '{}'",
		workspace.id
	))
	.fetch_one(setup.database())
	.await
	.expect("count query");
	assert_eq!(27, count, "a new workspace should seed 27 default roles");
}

#[tokio::test]
async fn create_role_name_too_short() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: "ab".to_string(),
						description: "too short".to_string(),
					},
					permissions: BTreeMap::from([(
						setup.get_permission_id(Permission::ViewRoles),
						ResourcePermissionType::Exclude(Default::default()),
					)]),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a role name shorter than 4 chars should be rejected, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn create_role_same_name_across_workspaces() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let name = random_name(8);

	for ws in [workspace_a.id, workspace_b.id] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<CreateNewRoleRequest>::builder()
					.path(CreateNewRolePath { workspace_id: ws })
					.headers(CreateNewRoleRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateNewRoleRequest {
						role: Role {
							name: name.clone(),
							description: "shared name".to_string(),
						},
						permissions: BTreeMap::from([(
							setup.get_permission_id(Permission::ViewRoles),
							ResourcePermissionType::Exclude(Default::default()),
						)]),
					})
					.build(),
			)
			.await;
		assert!(
			response.status_code().is_success(),
			"the same role name should be allowed in each workspace, got {}",
			response.status_code()
		);
	}
}

#[tokio::test]
async fn update_role_empty_permissions_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let role = setup
		.create_test_role(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
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
					role: Role {
						name: random_name(8),
						description: "test role".to_string(),
					},
					permissions: BTreeMap::new(),
				})
				.build(),
		)
		.await;
	assert_eq!(
		400,
		response.status_code().as_u16(),
		"a PATCH that empties the permissions map should be 400"
	);
}

#[tokio::test]
async fn update_user_roles_cross_workspace_role() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&admin.access_token).await;
	let workspace_b = setup.create_test_workspace(&admin.access_token).await;
	// A role that belongs to workspace B.
	let role_b = setup
		.create_test_role(&admin.access_token, workspace_b.id)
		.await;
	let user_b = setup.create_test_user().await;

	// Try to grant a workspace-B role to a user in workspace A.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: workspace_a.id,
					user_id: user_b.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest {
					roles: vec![role_b.id],
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"granting a role from another workspace should be rejected, got {}",
		response.status_code()
	);
}
