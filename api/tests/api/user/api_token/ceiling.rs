//! What a token may do. Its grants are a ceiling, intersected with the owner's
//! current permissions at auth time — so these cover minting limits, clamping
//! when the owner's roles change, and resource-scoped grants.

use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use models::{
	ApiSuccessResponseBody,
	api::{
		user::*,
		workspace::{
			LeaveWorkspacePath,
			LeaveWorkspaceRequest,
			LeaveWorkspaceRequestHeaders,
			LeaveWorkspaceResponse,
			deployment::*,
			rbac::{role::*, user::*},
		},
	},
	rbac::{DeploymentPermission, Permission, WorkspacePermission},
};

use super::{mint_token_raw, probe_modify_roles};
use crate::prelude::*;

/// A non-super-admin member cannot mint a super-admin token.
#[tokio::test]
async fn api_token_non_superadmin_cannot_mint_superadmin() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;

	let perms = vec![setup.get_permission_id(Permission::ViewRoles)];
	let role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, perms)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace.id, role.id)
		.await;

	let resp = mint_token_raw(
		&setup,
		&member.access_token,
		BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		None,
		None,
		None,
	)
	.await;
	assert!(
		resp.status_code().is_client_error(),
		"a member must not be able to mint a superAdmin token, got {}",
		resp.status_code()
	);
}

/// A token ceiling above its owner's permissions is allowed at mint time,
/// but the intersection at auth time clamps it: the token never acts beyond
/// its owner.
#[tokio::test]
async fn api_token_member_cannot_exceed_creator() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;

	// Member has only deployment::view.
	let view_role = setup
		.create_role_with_permissions(
			&owner.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Deployment(DeploymentPermission::View))],
		)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace.id, view_role.id)
		.await;

	// The member declares a ceiling carrying modifyRoles — allowed, since a
	// ceiling grants nothing by itself.
	let resp = mint_token_raw(
		&setup,
		&member.access_token,
		BTreeMap::from([(
			workspace.id,
			WorkspacePermission::Member {
				permissions: BTreeMap::from([(
					setup.get_permission_id(Permission::ModifyRoles),
					BTreeSet::from([workspace.id]),
				)]),
			},
		)]),
		None,
		None,
		None,
	)
	.await;
	assert!(
		resp.status_code().is_success(),
		"minting a ceiling above the owner's permissions must succeed, got {}",
		resp.status_code()
	);
	let token = resp
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response
		.token;

	// But acting on it is clamped by the owner's own permissions.
	let view_perm = setup.get_permission_id(Permission::ViewRoles);
	let probe = probe_modify_roles(&setup, &token, workspace.id, view_perm).await;
	assert!(
		probe.status_code().is_client_error(),
		"the token must not act beyond its owner's permissions, got {}",
		probe.status_code()
	);
}

#[tokio::test]
async fn api_token_patch_empty_permissions_400() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let token = setup
		.create_test_api_token(
			&owner.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath { token_id: token.id })
				.headers(UpdateApiTokenRequestHeaders {
					authorization: owner.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: "emptyperm".to_string(),
						permissions: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;
	assert_eq!(
		400,
		resp.status_code().as_u16(),
		"a PATCH with empty permissions should be 400"
	);
}

/// A token scoped to workspace A cannot access workspace B.
#[tokio::test]
async fn api_token_cannot_access_other_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&user_a.access_token).await;
	let user_b = setup.create_test_user().await;
	let ws_b = setup.create_test_workspace(&user_b.access_token).await;
	let token_a = setup
		.create_test_api_token(
			&user_a.access_token,
			BTreeMap::from([(ws_a.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let resp = setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: ws_b.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: BearerToken::from_str(&token_a.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		resp.status_code().is_client_error(),
		"a token scoped to workspace A must not access workspace B, got {}",
		resp.status_code()
	);
}

/// A token's permissions are trimmed when the holder's workspace roles are
/// stripped: a ModifyRoles probe goes from success to 401.
#[tokio::test]
async fn api_token_perm_trimmed_on_user_role_change() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let modify = setup.get_permission_id(Permission::ModifyRoles);
	let view = setup.get_permission_id(Permission::ViewRoles);

	let role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![modify])
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace.id, role.id)
		.await;
	let token = setup
		.create_test_api_token(
			&member.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(modify, BTreeSet::from([workspace.id]))]),
				},
			)]),
		)
		.await;

	assert!(
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.is_success(),
		"token should be able to modify roles before the trim"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: workspace.id,
					user_id: member.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: owner.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest { roles: vec![] })
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			UpdateUserRolesInWorkspaceResponse,
		));

	assert_eq!(
		401,
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.as_u16(),
		"after the role is stripped the token should lose ModifyRoles"
	);
}

/// Deleting the role (remove_users=true) that was a token holder's sole source
/// of a permission trims the token on its next use.
#[tokio::test]
async fn api_token_perm_trimmed_on_role_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let modify = setup.get_permission_id(Permission::ModifyRoles);
	let view = setup.get_permission_id(Permission::ViewRoles);

	let role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![modify])
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace.id, role.id)
		.await;
	let token = setup
		.create_test_api_token(
			&member.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(modify, BTreeSet::from([workspace.id]))]),
				},
			)]),
		)
		.await;

	assert!(
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.is_success()
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRoleRequest>::builder()
				.path(DeleteRolePath {
					workspace_id: workspace.id,
					role_id: role.id,
				})
				.query(DeleteRoleQuery { remove_users: true })
				.headers(DeleteRoleRequestHeaders {
					authorization: owner.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRoleResponse));

	assert_eq!(
		401,
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.as_u16(),
		"deleting the role with remove_users should trim the token"
	);
}

/// A token never outgrows its ceiling: promoting the member widens their own
/// permissions, but a token whose ceiling only carries the old role stays put.
/// (A ceiling that already carried the new permission WOULD widen — that is
/// `api_token_widens_up_to_ceiling_on_promotion`.)
#[tokio::test]
async fn api_token_does_not_widen_on_promotion() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let modify = setup.get_permission_id(Permission::ModifyRoles);
	let view = setup.get_permission_id(Permission::ViewRoles);

	let read_only = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![view])
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace.id, read_only.id)
		.await;
	let token = setup
		.create_test_api_token(
			&member.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(view, BTreeSet::from([workspace.id]))]),
				},
			)]),
		)
		.await;

	let write_role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![view, modify])
		.await;
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id: workspace.id,
					user_id: member.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: owner.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest {
					roles: vec![RoleBindingGrant {
						role_id: write_role.id,
						resource_id: workspace.id,
					}],
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			UpdateUserRolesInWorkspaceResponse,
		));

	assert_eq!(
		401,
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.as_u16(),
		"promotion must not widen a token whose ceiling lacks the permission"
	);
}

/// PATCHing a token to a narrower permission set revokes the dropped action.
#[tokio::test]
async fn api_token_patch_revokes_access() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let modify = setup.get_permission_id(Permission::ModifyRoles);
	let view = setup.get_permission_id(Permission::ViewRoles);

	let token = setup
		.create_test_api_token(
			&owner.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(modify, BTreeSet::from([workspace.id]))]),
				},
			)]),
		)
		.await;

	assert!(
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.is_success()
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath { token_id: token.id })
				.headers(UpdateApiTokenRequestHeaders {
					authorization: owner.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: "revoketoken".to_string(),
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::Member {
								permissions: BTreeMap::from([(
									view,
									BTreeSet::from([workspace.id]),
								)]),
							},
						)]),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateApiTokenResponse));

	assert_eq!(
		401,
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.as_u16(),
		"after the PATCH narrows perms the token should lose ModifyRoles"
	);
}

/// The auth-time intersection is per workspace, not just per permission: a
/// token keeps naming a workspace its owner has since been removed from, and
/// that workspace has to drop out of the token's effective permissions.
#[tokio::test]
async fn token_loses_workspace_when_owner_is_removed() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let modify = setup.get_permission_id(Permission::ModifyRoles);
	let view = setup.get_permission_id(Permission::ViewRoles);

	let role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![modify, view])
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace.id, role.id)
		.await;

	// The member's own workspace, where they are the super admin. It must be
	// untouched when they lose the other one.
	let own_workspace = setup.create_test_workspace(&member.access_token).await;

	let token = setup
		.create_test_api_token(
			&member.access_token,
			BTreeMap::from([
				(
					workspace.id,
					WorkspacePermission::Member {
						permissions: BTreeMap::from([(modify, BTreeSet::from([workspace.id]))]),
					},
				),
				(
					own_workspace.id,
					WorkspacePermission::Member {
						permissions: BTreeMap::from([(modify, BTreeSet::from([own_workspace.id]))]),
					},
				),
			]),
		)
		.await;

	assert!(
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.is_success(),
		"the member's token should work while they are still a member"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<RemoveUserFromWorkspaceRequest>::builder()
				.path(RemoveUserFromWorkspacePath {
					workspace_id: workspace.id,
					user_id: member.user_id,
				})
				.headers(RemoveUserFromWorkspaceRequestHeaders {
					authorization: owner.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(RemoveUserFromWorkspaceRequest)
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			RemoveUserFromWorkspaceResponse,
		));

	assert_eq!(
		401,
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.as_u16(),
		"the token still names the workspace, but the owner is no longer a member"
	);

	assert!(
		probe_modify_roles(&setup, &token.token, own_workspace.id, view)
			.await
			.status_code()
			.is_success(),
		"losing one workspace must not touch the token's other workspaces"
	);
}

/// Same drop, member-initiated: leaving a workspace has to take the token's
/// access to it with them.
#[tokio::test]
async fn token_loses_workspace_when_owner_leaves() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let modify = setup.get_permission_id(Permission::ModifyRoles);
	let view = setup.get_permission_id(Permission::ViewRoles);

	let role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![modify, view])
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&owner.access_token, workspace.id, role.id)
		.await;

	let token = setup
		.create_test_api_token(
			&member.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(modify, BTreeSet::from([workspace.id]))]),
				},
			)]),
		)
		.await;

	assert!(
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.is_success()
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<LeaveWorkspaceRequest>::builder()
				.path(LeaveWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(LeaveWorkspaceRequestHeaders {
					authorization: member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(LeaveWorkspaceRequest)
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(LeaveWorkspaceResponse));

	assert_eq!(
		401,
		probe_modify_roles(&setup, &token.token, workspace.id, view)
			.await
			.status_code()
			.as_u16(),
		"leaving the workspace must drop it from the token's effective permissions"
	);
}

#[tokio::test]
async fn use_api_token_for_auth() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	// Use the API token to list workspaces — should succeed since it auths as
	// the owning user.
	let response = setup
		.make_api_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	assert!(
		response
			.response
			.workspaces
			.iter()
			.any(|w| w.id == workspace.id),
		"workspace should be visible via API token"
	);
}

#[tokio::test]
async fn use_revoked_api_token() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeApiTokenRequest>::builder()
				.path(RevokeApiTokenPath {
					token_id: api_token.id,
				})
				.headers(RevokeApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevokeApiTokenResponse));

	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();
	let response = setup
		.make_api_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"revoked API token should be rejected, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_with_scoped_permissions_allows_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let _deployment2 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let view_perm = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(view_perm, BTreeSet::from([deployment1.id]))]),
				},
			)]),
		)
		.await;
	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	let response = setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment1.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"included deployment should be readable, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_with_scoped_permissions_denies_other_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment1 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let deployment2 = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let view_perm = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(view_perm, BTreeSet::from([deployment1.id]))]),
				},
			)]),
		)
		.await;
	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	let response = setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment2.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-included deployment should be denied, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_view_permission_denies_create() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let view_perm = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			// grants View on all
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(view_perm, BTreeSet::from([workspace.id]))]),
				},
			)]),
		)
		.await;
	let token_bearer = BearerToken::from_str(&api_token.token).unwrap();

	// Pick any machine type to satisfy the request body schema.
	use models::api::workspace::deployment::{
		ListAllDeploymentMachineTypePath,
		ListAllDeploymentMachineTypeRequest,
		ListAllDeploymentMachineTypeRequestHeaders,
		ListAllDeploymentMachineTypeResponse,
	};
	let machine_types = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath {
					workspace_id: workspace.id,
				})
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();
	let machine_type_id = machine_types.response.machine_types[0].id;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: runner.id,
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
		.await;

	assert!(
		response.status_code().is_client_error(),
		"View-only token should be denied Create, got {}",
		response.status_code()
	);
}
