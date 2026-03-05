use std::collections::BTreeMap;

use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::rbac::{*, role::*, user::*},
	},
	rbac::ResourcePermissionType,
};

use crate::prelude::*;

#[tokio::test]
async fn list_all_permissions_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListAllPermissionsRequest::METHOD,
			&ListAllPermissionsPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
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
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListAllResourceTypesRequest::METHOD,
			&ListAllResourceTypesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
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
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetCurrentPermissionsRequest::METHOD,
			&GetCurrentPermissionsPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
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
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let role = create_test_role(&setup, &user.access_token, ws.id).await;
	assert!(!role.name.is_empty());
}

#[tokio::test]
async fn list_roles_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let _role = create_test_role(&setup, &user.access_token, ws.id).await;

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
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListAllRolesResponse>>();

	assert!(!response.response.roles.is_empty());
}

#[tokio::test]
async fn get_role_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let role = create_test_role(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			GetRoleInfoRequest::METHOD,
			&GetRoleInfoPath {
				workspace_id: ws.id,
				role_id: role.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetRoleInfoResponse>>();

	assert_eq!(role.name, response.response.role.name);
}

#[tokio::test]
async fn update_role_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let role = create_test_role(&setup, &user.access_token, ws.id).await;
	let new_name = random_name(8);

	setup
		.server
		.method(
			UpdateRoleRequest::METHOD,
			&UpdateRolePath {
				workspace_id: ws.id,
				role_id: role.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&UpdateRoleRequest {
			name: Some(new_name.clone()),
			description: None,
			permissions: None,
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateRoleResponse));
}

#[tokio::test]
async fn delete_role_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let role = create_test_role(&setup, &user.access_token, ws.id).await;

	let path = format!(
		"{}?remove_users=false",
		DeleteRolePath {
			workspace_id: ws.id,
			role_id: role.id,
		}
		.to_string()
	);

	setup
		.server
		.method(DeleteRoleRequest::METHOD, &path)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRoleResponse));
}

#[tokio::test]
async fn list_users_for_role_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let role = create_test_role(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			ListUsersForRoleRequest::METHOD,
			&ListUsersForRolePath {
				workspace_id: ws.id,
				role_id: role.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListUsersForRoleResponse>>();

	// New role, no users assigned yet
	assert!(response.response.users.is_empty());
}

#[tokio::test]
async fn list_users_in_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListUsersInWorkspaceRequest::METHOD,
			&ListUsersInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
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
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let role = create_test_role(&setup, &admin.access_token, ws.id).await;
	let user_b =
		add_user_to_workspace_with_role(&setup, &admin.access_token, ws.id, role.id)
			.await;

	// Verify user B is in the workspace
	let response = setup
		.server
		.method(
			ListUsersInWorkspaceRequest::METHOD,
			&ListUsersInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&admin.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListUsersInWorkspaceResponse>>();

	assert!(response.response.users.contains_key(&user_b.user_id));
}

#[tokio::test]
async fn remove_user_from_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let role = create_test_role(&setup, &admin.access_token, ws.id).await;
	let user_b =
		add_user_to_workspace_with_role(&setup, &admin.access_token, ws.id, role.id)
			.await;

	setup
		.server
		.method(
			RemoveUserFromWorkspaceRequest::METHOD,
			&RemoveUserFromWorkspacePath {
				workspace_id: ws.id,
				user_id: user_b.user_id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&admin.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			RemoveUserFromWorkspaceResponse,
		));

	// Verify user B is gone
	let response = setup
		.server
		.method(
			ListUsersInWorkspaceRequest::METHOD,
			&ListUsersInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&admin.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListUsersInWorkspaceResponse>>();

	assert!(!response.response.users.contains_key(&user_b.user_id));
}

#[tokio::test]
async fn get_current_permissions_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;

	let perm_ids = get_all_permission_ids(&setup, &admin.access_token, ws.id).await;
	let view_roles_id = perm_ids
		.get("viewRoles")
		.expect("viewRoles permission not found");

	let mut permissions = BTreeMap::new();
	permissions.insert(
		*view_roles_id,
		ResourcePermissionType::Exclude(std::collections::BTreeSet::new()),
	);

	let role = create_role_with_permissions(
		&setup,
		&admin.access_token,
		ws.id,
		permissions,
	)
	.await;

	let user_b =
		add_user_to_workspace_with_role(&setup, &admin.access_token, ws.id, role.id)
			.await;

	let response = setup
		.server
		.method(
			GetCurrentPermissionsRequest::METHOD,
			&GetCurrentPermissionsPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user_b.access_token)
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
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

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
		.await;

	assert!(response.status_code().is_client_error());
}
