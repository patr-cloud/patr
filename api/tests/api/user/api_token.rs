use std::{
	collections::{BTreeMap, BTreeSet},
	net::IpAddr,
	str::FromStr,
};

use ipnetwork::IpNetwork;
use models::{
	ApiSuccessResponseBody,
	api::{
		user::*,
		workspace::{
			deployment::*,
			rbac::{role::*, user::*},
		},
	},
	rbac::{
		DeploymentPermission,
		Permission,
		PermissionScope,
		ResourcePermissionType,
		WorkspacePermission,
	},
	utils::{ListResourceQuery, Uuid},
};

use crate::prelude::*;

/// Probe a ModifyRoles-gated action (create a role) using a raw API token.
/// Returns the raw response so callers can assert the authz outcome.
async fn probe_modify_roles(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
	view_perm: Uuid,
) -> axum_test::TestResponse {
	setup
		.make_api_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath { workspace_id })
				.headers(CreateNewRoleRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: "cascade probe".to_string(),
					},
					permissions: vec![view_perm],
				})
				.build(),
		)
		.await
}

/// Call the API-token entrypoint with the given raw token (lists workspaces).
/// Returns the raw response so callers can assert the auth outcome.
async fn call_with_token(setup: &TestSetup, token: &str) -> axum_test::TestResponse {
	setup
		.make_api_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
}

/// Mint an API token via the web dashboard, returning the raw response.
async fn mint_token_raw(
	setup: &TestSetup,
	token: &BearerToken,
	super_admin_of: BTreeSet<Uuid>,
	grants: BTreeMap<Uuid, Vec<RoleGrant>>,
	token_nbf: Option<time::OffsetDateTime>,
	token_exp: Option<time::OffsetDateTime>,
	allowed_ips: Option<Vec<ipnetwork::IpNetwork>>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						super_admin_of,
						grants,
						token_nbf,
						token_exp,
						allowed_ips,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
}

/// A role permission grant over all resources (Exclude of the empty set).
fn all_resources() -> ResourcePermissionType {
	ResourcePermissionType::Exclude(BTreeSet::new())
}

/// A token permission scope covering the whole workspace.
fn workspace_scope() -> PermissionScope {
	PermissionScope::Workspace
}

/// A one-role token grant map for a workspace.
fn role_grants(workspace_id: Uuid, role_id: Uuid, scope: PermissionScope) -> BTreeMap<Uuid, Vec<RoleGrant>> {
	BTreeMap::from([(workspace_id, vec![RoleGrant { role_id, scope }])])
}

/// A token used after its `token_exp` is rejected at auth time (401).
#[tokio::test]
async fn api_token_expired_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeSet::from([workspace.id]),
		BTreeMap::new(),
		None,
		Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert_eq!(
		401,
		call_with_token(&setup, &token).await.status_code().as_u16(),
		"an expired token should be rejected with 401"
	);
}

/// A token used before its `token_nbf` is rejected at auth time (401).
#[tokio::test]
async fn api_token_before_nbf_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeSet::from([workspace.id]),
		BTreeMap::new(),
		Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
		None,
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert_eq!(
		401,
		call_with_token(&setup, &token).await.status_code().as_u16(),
		"a token used before its NBF should be rejected with 401"
	);
}

/// A token whose NBF is now and EXP is far in the future is accepted.
#[tokio::test]
async fn api_token_valid_nbf_exp_accepted() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeSet::from([workspace.id]),
		BTreeMap::new(),
		Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(7)),
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert!(
		call_with_token(&setup, &token)
			.await
			.status_code()
			.is_success(),
		"a token within its NBF..EXP window should be accepted"
	);
}

/// Minting a token with NBF later than EXP is rejected (400).
#[tokio::test]
async fn api_token_nbf_after_exp_rejected_on_create() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let resp = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeSet::from([workspace.id]),
		BTreeMap::new(),
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(7)),
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(1)),
		None,
	)
	.await;
	assert_eq!(
		400,
		resp.status_code().as_u16(),
		"minting a token with NBF > EXP should be 400"
	);
}

/// A PATCH that lands the token in NBF > EXP is rejected (400).
#[tokio::test]
async fn api_token_nbf_after_exp_rejected_on_patch() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let id = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeSet::from([workspace.id]),
		BTreeMap::new(),
		None,
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(1)),
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.id;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath { token_id: id })
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: "patchtoken".to_string(),
						super_admin_of: BTreeSet::from([workspace.id]),
						grants: BTreeMap::new(),
						// nbf 7 days out, exp 1 day out (resent) → nbf > exp → 400.
						token_nbf: Some(time::OffsetDateTime::now_utc() + time::Duration::days(7)),
						token_exp: Some(time::OffsetDateTime::now_utc() + time::Duration::days(1)),
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
		"a PATCH landing NBF > EXP should be 400"
	);
}

/// A token created with an empty `allowed_ips` list is callable (empty list is
/// normalized to "no whitelist", not "block all").
#[tokio::test]
async fn api_token_empty_allowed_ips_callable() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeSet::from([workspace.id]),
		BTreeMap::new(),
		None,
		None,
		Some(vec![]),
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert!(
		call_with_token(&setup, &token)
			.await
			.status_code()
			.is_success(),
		"empty allowed_ips should not block the token"
	);
}

/// A malformed token is rejected with 400.
#[tokio::test]
async fn api_token_malformed_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	assert_eq!(
		400,
		call_with_token(&setup, "patrv1.garbage")
			.await
			.status_code()
			.as_u16(),
		"a malformed token should be 400"
	);
}

/// A well-formed but unknown token is rejected with 401.
#[tokio::test]
async fn api_token_unknown_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let fake = format!("patrv1.{}.{}", Uuid::nil(), Uuid::nil());
	assert_eq!(
		401,
		call_with_token(&setup, &fake).await.status_code().as_u16(),
		"a well-formed but unknown token should be 401"
	);
}

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
		BTreeSet::from([workspace.id]),
		BTreeMap::new(),
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

	// Member has only deployment::view; a second role carries modifyRoles.
	let view_role = setup
		.create_role_with_permissions(
			&owner.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Deployment(DeploymentPermission::View))],
		)
		.await;
	let modify_role = setup
		.create_role_with_permissions(
			&owner.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ModifyRoles)],
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
		BTreeSet::new(),
		role_grants(workspace.id, modify_role.id, workspace_scope()),
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
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
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
						super_admin_of: BTreeSet::new(),
						grants: BTreeMap::new(),
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

/// One user cannot delete another user's token (404), and the victim's token
/// keeps working.
#[tokio::test]
async fn api_token_cross_user_delete_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&user_a.access_token).await;
	let user_b = setup.create_test_user().await;
	let token_a = setup
		.create_test_api_token(
			&user_a.access_token,
			BTreeSet::from([ws_a.id]),
			BTreeMap::new(),
		)
		.await;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeApiTokenRequest>::builder()
				.path(RevokeApiTokenPath {
					token_id: token_a.id,
				})
				.headers(RevokeApiTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		404,
		resp.status_code().as_u16(),
		"deleting another user's token should be 404"
	);

	assert!(
		call_with_token(&setup, &token_a.token)
			.await
			.status_code()
			.is_success(),
		"the victim's token should still work"
	);
}

/// One user cannot regenerate another user's token (404).
#[tokio::test]
async fn api_token_cross_user_regenerate_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&user_a.access_token).await;
	let user_b = setup.create_test_user().await;
	let token_a = setup
		.create_test_api_token(
			&user_a.access_token,
			BTreeSet::from([ws_a.id]),
			BTreeMap::new(),
		)
		.await;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateApiTokenRequest>::builder()
				.path(RegenerateApiTokenPath {
					token_id: token_a.id,
				})
				.headers(RegenerateApiTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		404,
		resp.status_code().as_u16(),
		"regenerating another user's token should be 404"
	);
}

/// A PATCH targeting another user's token (IDOR) is 404 and does not wipe the
/// victim's permissions.
#[tokio::test]
async fn api_token_cross_user_patch_idor_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&user_a.access_token).await;
	let user_b = setup.create_test_user().await;
	let token_a = setup
		.create_test_api_token(
			&user_a.access_token,
			BTreeSet::from([ws_a.id]),
			BTreeMap::new(),
		)
		.await;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: token_a.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: "idortoken".to_string(),
						super_admin_of: BTreeSet::from([ws_a.id]),
						grants: BTreeMap::new(),
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
		404,
		resp.status_code().as_u16(),
		"PATCHing another user's token (IDOR) should be 404"
	);

	assert!(
		call_with_token(&setup, &token_a.token)
			.await
			.status_code()
			.is_success(),
		"the victim's token should still work after the IDOR attempt"
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
			BTreeSet::from([ws_a.id]),
			BTreeMap::new(),
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
			BTreeSet::new(),
			role_grants(workspace.id, role.id, workspace_scope()),
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
			BTreeSet::new(),
			role_grants(workspace.id, role.id, workspace_scope()),
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
			BTreeSet::new(),
			role_grants(workspace.id, read_only.id, workspace_scope()),
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
					roles: vec![RoleGrant {
						role_id: write_role.id,
						scope: PermissionScope::Workspace,
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

	let write_role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![view, modify])
		.await;
	let view_role = setup
		.create_role_with_permissions(&owner.access_token, workspace.id, vec![view])
		.await;
	let token = setup
		.create_test_api_token(
			&owner.access_token,
			BTreeSet::new(),
			role_grants(workspace.id, write_role.id, workspace_scope()),
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
						super_admin_of: BTreeSet::new(),
						grants: role_grants(workspace.id, view_role.id, workspace_scope()),
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

/// A token name frees up once the token is revoked and can be reused.
#[tokio::test]
async fn api_token_name_reusable_after_revoke() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let name = random_name(8);
	let super_admins = BTreeSet::from([workspace.id]);

	let first = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: name.clone(),
						super_admin_of: super_admins.clone(),
						grants: BTreeMap::new(),
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

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeApiTokenRequest>::builder()
				.path(RevokeApiTokenPath { token_id: first.id })
				.headers(RevokeApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevokeApiTokenResponse));

	let second = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name,
						super_admin_of: super_admins.clone(),
						grants: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;
	assert!(
		second.status_code().is_success(),
		"a token name should be reusable after the previous token is revoked, got {}",
		second.status_code()
	);
}

#[tokio::test]
async fn create_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;
	assert!(!api_token.token.is_empty(), "token should not be empty");
}

#[tokio::test]
async fn list_api_tokens_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let super_admins = BTreeSet::from([workspace.id]);

	let _t1 = setup
		.create_test_api_token(&user.access_token, super_admins.clone(), BTreeMap::new())
		.await;
	let _t2 = setup.create_test_api_token(&user.access_token, super_admins, BTreeMap::new()).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.headers(ListApiTokensRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListApiTokensResponse>>();

	assert!(
		response.response.tokens.len() >= 2,
		"should have at least 2 tokens"
	);
}

#[tokio::test]
async fn get_api_token_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: api_token.id,
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetApiTokenInfoResponse>>();

	assert_eq!(api_token.name, response.response.token.name);
}

#[tokio::test]
async fn update_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;
	let new_name = random_name(8);

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: api_token.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: new_name.clone(),
						super_admin_of: BTreeSet::from([workspace.id]),
						grants: BTreeMap::new(),
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

	// Verify the update
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: api_token.id,
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetApiTokenInfoResponse>>();

	assert_eq!(new_name, response.response.token.name);
}

#[tokio::test]
async fn revoke_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
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

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: api_token.id,
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"revoked token should not be found"
	);
}

#[tokio::test]
async fn regenerate_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateApiTokenRequest>::builder()
				.path(RegenerateApiTokenPath {
					token_id: api_token.id,
				})
				.headers(RegenerateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RegenerateApiTokenResponse>>();

	assert_ne!(
		api_token.token, response.response.token,
		"regenerated token should be different"
	);
}

#[tokio::test]
async fn get_api_token_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiTokenInfoRequest>::builder()
				.path(GetApiTokenInfoPath {
					token_id: Uuid::nil(),
				})
				.headers(GetApiTokenInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent token"
	);
}

#[tokio::test]
async fn create_api_token_with_empty_permissions_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						super_admin_of: BTreeSet::new(),
						grants: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"creating an API token with empty permissions should fail, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.headers(ListApiTokensRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}

#[tokio::test]
async fn create_api_token_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let super_admins = BTreeSet::from([workspace.id]);

	let first = setup
		.create_test_api_token(&user.access_token, super_admins.clone(), BTreeMap::new())
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: first.name.clone(),
						super_admin_of: super_admins.clone(),
						grants: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate token name, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn update_api_token_name_conflict() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let super_admins = BTreeSet::from([workspace.id]);

	let first = setup
		.create_test_api_token(&user.access_token, super_admins.clone(), BTreeMap::new())
		.await;
	let second = setup.create_test_api_token(&user.access_token, super_admins, BTreeMap::new()).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: second.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: first.name.clone(),
						super_admin_of: BTreeSet::from([workspace.id]),
						grants: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"renaming a token to a name already in use should fail, got {}",
		response.status_code()
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
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
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
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
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
async fn list_api_tokens_pagination() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let super_admins = BTreeSet::from([workspace.id]);

	for _ in 0..3 {
		setup
			.create_test_api_token(&user.access_token, super_admins.clone(), BTreeMap::new())
			.await;
	}

	let page0 = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 2,
					page: 0,
					additional_query: (),
				})
				.headers(ListApiTokensRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListApiTokensResponse>>();
	assert_eq!(
		page0.response.tokens.len(),
		2,
		"page 0 should have 2 tokens"
	);

	let page1 = setup
		.make_web_dashboard_call(
			ApiRequest::<ListApiTokensRequest>::builder()
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 2,
					page: 1,
					additional_query: (),
				})
				.headers(ListApiTokensRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListApiTokensResponse>>();
	assert!(
		page1.response.tokens.len() >= 1,
		"page 1 should have remaining token(s)"
	);

	// Pages must not overlap.
	let page0_ids: BTreeSet<Uuid> = page0.response.tokens.iter().map(|t| t.id).collect();
	let page1_ids: BTreeSet<Uuid> = page1.response.tokens.iter().map(|t| t.id).collect();
	assert!(
		page0_ids.is_disjoint(&page1_ids),
		"pages should not contain overlapping tokens"
	);
}

#[tokio::test]
async fn api_token_with_ip_restriction_allows_listed_ip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let allowed: IpAddr = "1.2.3.4".parse().unwrap();
	let create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						super_admin_of: BTreeSet::from([workspace.id]),
						grants: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: Some(vec![IpNetwork::from(allowed)]),
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response;

	let token_bearer = BearerToken::from_str(&create.token).unwrap();
	let response = setup
		.make_api_call_from_ip(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			allowed,
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"request from allowed IP should succeed, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_with_ip_restriction_blocks_unlisted_ip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let allowed: IpAddr = "1.2.3.4".parse().unwrap();
	let blocked: IpAddr = "5.6.7.8".parse().unwrap();
	let create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						super_admin_of: BTreeSet::from([workspace.id]),
						grants: BTreeMap::new(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: Some(vec![IpNetwork::from(allowed)]),
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response;

	let token_bearer = BearerToken::from_str(&create.token).unwrap();
	let response = setup
		.make_api_call_from_ip(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			blocked,
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"request from disallowed IP should be rejected, got {}",
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
	let scoped_role = setup
		.create_role_with_permissions(&user.access_token, workspace.id, vec![view_perm])
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::new(),
			role_grants(workspace.id, scoped_role.id, PermissionScope::Resources(BTreeSet::from([deployment1.id]))),
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
	let scoped_role = setup
		.create_role_with_permissions(&user.access_token, workspace.id, vec![view_perm])
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::new(),
			role_grants(workspace.id, scoped_role.id, PermissionScope::Resources(BTreeSet::from([deployment1.id]))),
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
	let view_role = setup
		.create_role_with_permissions(&user.access_token, workspace.id, vec![view_perm])
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::new(),
			// grants View on all
			role_grants(workspace.id, view_role.id, workspace_scope()),
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
