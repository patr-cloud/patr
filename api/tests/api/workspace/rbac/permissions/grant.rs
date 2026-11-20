use std::collections::{BTreeMap, BTreeSet};

use models::{
	ApiErrorResponseBody,
	api::workspace::rbac::{role::*, user::*},
	rbac::{Permission, SecretPermission, WorkspacePermission},
	utils::{ListResourceQuery, Uuid},
};

use super::grants;
use crate::prelude::*;

/// Assert that `response` was refused for granting more than its caller holds.
fn assert_exceeds(response: &axum_test::TestResponse, message: &str) {
	assert_eq!(403, response.status_code().as_u16(), "{message}");
	assert_eq!(
		ErrorType::GrantExceedsPermissions,
		response.json::<ApiErrorResponseBody>().error,
		"{message}"
	);
}

/// An admin, their workspace, a role holding `ModifyRoles` and `ViewRoles`,
/// and a member bound to that role across the workspace.
async fn workspace_with_role_manager(setup: &TestSetup) -> (TestUser, Uuid, TestRole, TestUser) {
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let manage = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ModifyRoles),
				setup.get_permission_id(Permission::ViewRoles),
			],
		)
		.await;
	let manager = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, manage.id)
		.await;

	(admin, workspace.id, manage, manager)
}

/// Create a role named at random holding `permissions`, as `token`.
async fn create_role(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	permissions: &[Permission],
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath { workspace_id })
				.headers(CreateNewRoleRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test".to_string(),
						is_immutable: false,
					},
					permissions: permissions
						.iter()
						.map(|permission| setup.get_permission_id(*permission))
						.collect(),
				})
				.build(),
		)
		.await
}

/// Replace the permissions of `role_id` with `permissions`, renaming it at
/// random, as `token`.
async fn update_role(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	role_id: Uuid,
	permissions: &[Permission],
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateRoleRequest>::builder()
				.path(UpdateRolePath {
					workspace_id,
					role_id,
				})
				.headers(UpdateRoleRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test".to_string(),
						is_immutable: false,
					},
					permissions: permissions
						.iter()
						.map(|permission| setup.get_permission_id(*permission))
						.collect(),
				})
				.build(),
		)
		.await
}

/// The permissions `role_id` holds, as `token` sees them.
async fn permissions_of(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	role_id: Uuid,
) -> BTreeSet<Uuid> {
	setup
		.make_web_dashboard_call(
			ApiRequest::<GetRoleInfoRequest>::builder()
				.path(GetRoleInfoPath {
					workspace_id,
					role_id,
				})
				.headers(GetRoleInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetRoleInfoResponse>>()
		.response
		.permissions
}

/// Replace the bindings of `user_id` with `roles`, as `token`.
async fn update_member(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	user_id: Uuid,
	roles: Vec<RoleBindingGrant>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id,
					user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest { roles })
				.build(),
		)
		.await
}

/// The bindings `user_id` holds, as `token` sees them.
async fn bindings_of(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	user_id: Uuid,
) -> BTreeSet<RoleBindingGrant> {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListUsersInWorkspaceRequest>::builder()
				.path(ListUsersInWorkspacePath { workspace_id })
				.headers(ListUsersInWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUsersInWorkspaceResponse>>()
		.response
		.users
		.into_iter()
		.find(|member| member.user.id == user_id)
		.expect("the user should be a member of the workspace")
		.role_bindings
		.into_iter()
		.collect()
}

/// Invite a random email to the workspace with `roles`, as `token`.
async fn invite(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	roles: Vec<RoleBindingGrant>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<InviteUserToWorkspaceRequest>::builder()
				.path(InviteUserToWorkspacePath { workspace_id })
				.headers(InviteUserToWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(InviteUserToWorkspaceRequest {
					email: format!("{}@example.com", random_name(12)),
					roles,
				})
				.build(),
		)
		.await
}

/// Replace the grants of `invite_id` with `roles`, as `token`.
async fn update_invite(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	invite_id: Uuid,
	roles: Vec<RoleBindingGrant>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateWorkspaceInviteRolesRequest>::builder()
				.path(UpdateWorkspaceInviteRolesPath {
					workspace_id,
					invite_id,
				})
				.headers(UpdateWorkspaceInviteRolesRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInviteRolesRequest { roles })
				.build(),
		)
		.await
}

/// Resend `invite_id`, as `token`.
async fn resend_invite(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	invite_id: Uuid,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ResendWorkspaceInviteRequest>::builder()
				.path(ResendWorkspaceInvitePath {
					workspace_id,
					invite_id,
				})
				.headers(ResendWorkspaceInviteRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
}

/// The grants `invite_id` carries, as `token` sees them.
async fn grants_of_invite(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	invite_id: Uuid,
) -> BTreeSet<RoleBindingGrant> {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListWorkspaceInvitesRequest>::builder()
				.path(ListWorkspaceInvitesPath { workspace_id })
				.headers(ListWorkspaceInvitesRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListWorkspaceInvitesResponse>>()
		.response
		.invites
		.into_iter()
		.find(|invite| invite.id == invite_id)
		.expect("the invite should be pending")
		.data
		.roles
		.into_iter()
		.collect()
}

/// A role can't be created holding a permission its author lacks. The super
/// admin creates anything.
#[tokio::test]
async fn role_create_cannot_carry_a_permission_the_caller_lacks() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, _manage, manager) = workspace_with_role_manager(&setup).await;

	let refused = create_role(
		&setup,
		&manager.access_token,
		workspace_id,
		&[
			Permission::ViewRoles,
			Permission::Secret(SecretPermission::View),
		],
	)
	.await;
	assert_exceeds(
		&refused,
		"a role holding a permission its author lacks should not be creatable",
	);

	let allowed = create_role(
		&setup,
		&manager.access_token,
		workspace_id,
		&[Permission::ViewRoles, Permission::ModifyRoles],
	)
	.await;
	assert!(
		allowed.status_code().is_success(),
		"a role holding only what its author holds should be creatable, got {}",
		allowed.status_code()
	);

	let allowed = create_role(
		&setup,
		&admin.access_token,
		workspace_id,
		&[Permission::Secret(SecretPermission::View)],
	)
	.await;
	assert!(
		allowed.status_code().is_success(),
		"the super admin should create a role holding anything, got {}",
		allowed.status_code()
	);
}

/// A role can be bound anywhere, so a permission held on one resource isn't
/// enough to write it into a role.
#[tokio::test]
async fn role_create_needs_the_permission_across_the_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, _manage, manager) = workspace_with_role_manager(&setup).await;
	let secret = setup
		.create_test_secret(&admin.access_token, workspace_id)
		.await;
	let view = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::Edit))],
		)
		.await;
	let own = bindings_of(&setup, &admin.access_token, workspace_id, manager.user_id).await;
	update_member(
		&setup,
		&admin.access_token,
		workspace_id,
		manager.user_id,
		own.into_iter()
			.chain(grants(view.id, &[secret.id]))
			.collect(),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));

	let refused = create_role(
		&setup,
		&manager.access_token,
		workspace_id,
		&[Permission::Secret(SecretPermission::Edit)],
	)
	.await;
	assert_exceeds(
		&refused,
		"a permission held on one resource should not be writable into a role",
	);
}

/// Only what an edit adds to a role is checked: a permission the editor lacks
/// survives a rename and can be removed, but can't be put back.
#[tokio::test]
async fn role_update_checks_only_added_permissions() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, _manage, manager) = workspace_with_role_manager(&setup).await;
	let existing = [
		Permission::ViewRoles,
		Permission::Secret(SecretPermission::View),
	];
	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			existing
				.iter()
				.map(|permission| setup.get_permission_id(*permission))
				.collect(),
		)
		.await;

	update_role(
		&setup,
		&manager.access_token,
		workspace_id,
		role.id,
		&existing,
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateRoleResponse));
	assert_eq!(
		existing
			.iter()
			.map(|permission| setup.get_permission_id(*permission))
			.collect::<BTreeSet<_>>(),
		permissions_of(&setup, &admin.access_token, workspace_id, role.id).await,
		"a permission the editor lacks should survive an edit that leaves it alone"
	);

	let refused = update_role(
		&setup,
		&manager.access_token,
		workspace_id,
		role.id,
		&[
			Permission::ViewRoles,
			Permission::Secret(SecretPermission::View),
			Permission::Secret(SecretPermission::Edit),
		],
	)
	.await;
	assert_exceeds(
		&refused,
		"a permission the editor lacks should not be addable to a role",
	);

	update_role(
		&setup,
		&manager.access_token,
		workspace_id,
		role.id,
		&[Permission::ViewRoles],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateRoleResponse));

	let refused = update_role(
		&setup,
		&manager.access_token,
		workspace_id,
		role.id,
		&existing,
	)
	.await;
	assert_exceeds(
		&refused,
		"a permission the editor removed should not be theirs to put back",
	);
	assert_eq!(
		BTreeSet::from([setup.get_permission_id(Permission::ViewRoles)]),
		permissions_of(&setup, &admin.access_token, workspace_id, role.id).await,
		"a refused edit should leave the role as it was"
	);

	update_role(
		&setup,
		&admin.access_token,
		workspace_id,
		role.id,
		&existing,
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateRoleResponse));
}

/// Binding a role you cover and then widening the role is refused at the
/// edit.
#[tokio::test]
async fn role_bound_to_its_author_cannot_be_widened() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;

	let role_id = create_role(
		&setup,
		&manager.access_token,
		workspace_id,
		&[Permission::ViewRoles],
	)
	.await
	.json::<ApiSuccessResponseBody<CreateNewRoleResponse>>()
	.response
	.id
	.id;
	update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		manager.user_id,
		vec![
			RoleBindingGrant {
				role_id: manage.id,
				resource_id: workspace_id,
			},
			RoleBindingGrant {
				role_id,
				resource_id: workspace_id,
			},
		],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));

	let refused = update_role(
		&setup,
		&manager.access_token,
		workspace_id,
		role_id,
		&[Permission::ViewRoles, Permission::EditWorkspace],
	)
	.await;
	assert_exceeds(
		&refused,
		"a role bound to its author should not be widened past what they hold",
	);
	assert_eq!(
		BTreeSet::from([setup.get_permission_id(Permission::ViewRoles)]),
		permissions_of(&setup, &admin.access_token, workspace_id, role_id).await
	);
}

/// A member can't be handed a role the caller doesn't hold, and the refused
/// update leaves their bindings as they were.
#[tokio::test]
async fn member_update_cannot_grant_a_role_the_caller_lacks() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let view = setup
		.create_test_role(&admin.access_token, workspace_id)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace_id, view.id)
		.await;
	let before = bindings_of(&setup, &admin.access_token, workspace_id, member.user_id).await;

	let refused = update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		grants(not_held.id, &[workspace_id]),
	)
	.await;
	assert_exceeds(
		&refused,
		"a role the caller doesn't hold should not be grantable",
	);
	assert_eq!(
		before,
		bindings_of(&setup, &admin.access_token, workspace_id, member.user_id).await,
		"a refused update should leave the bindings as they were"
	);

	// Nor can the caller hand it to themselves.
	let refused = update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		manager.user_id,
		vec![
			RoleBindingGrant {
				role_id: manage.id,
				resource_id: workspace_id,
			},
			RoleBindingGrant {
				role_id: not_held.id,
				resource_id: workspace_id,
			},
		],
	)
	.await;
	assert_exceeds(
		&refused,
		"a role the caller doesn't hold should not be self-grantable",
	);

	update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		grants(manage.id, &[workspace_id]),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));

	update_member(
		&setup,
		&admin.access_token,
		workspace_id,
		member.user_id,
		grants(not_held.id, &[workspace_id]),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));
}

/// A role held on one resource can be passed on at that resource, and no
/// wider: not at another resource, and not across the workspace.
#[tokio::test]
async fn member_grant_cannot_be_wider_than_the_callers_scope() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let held_at = setup
		.create_test_secret(&admin.access_token, workspace_id)
		.await;
	let other = setup
		.create_test_secret(&admin.access_token, workspace_id)
		.await;
	let view = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::Edit))],
		)
		.await;
	update_member(
		&setup,
		&admin.access_token,
		workspace_id,
		manager.user_id,
		vec![
			RoleBindingGrant {
				role_id: manage.id,
				resource_id: workspace_id,
			},
			RoleBindingGrant {
				role_id: view.id,
				resource_id: held_at.id,
			},
		],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace_id, manage.id)
		.await;

	for scope in [workspace_id, other.id] {
		let refused = update_member(
			&setup,
			&manager.access_token,
			workspace_id,
			member.user_id,
			grants(view.id, &[scope]),
		)
		.await;
		assert_exceeds(
			&refused,
			&format!("a role held on one resource should not be grantable at `{scope}`"),
		);
	}

	update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		grants(view.id, &[held_at.id]),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));
}

/// A binding the editor couldn't make themselves survives their update as
/// long as it's sent back untouched. They can drop it, but can't re-scope it
/// or put it back.
#[tokio::test]
async fn member_update_keeps_a_grant_the_caller_could_not_make() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let secret = setup
		.create_test_secret(&admin.access_token, workspace_id)
		.await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace_id, not_held.id)
		.await;
	let existing = RoleBindingGrant {
		role_id: not_held.id,
		resource_id: workspace_id,
	};
	let added = RoleBindingGrant {
		role_id: manage.id,
		resource_id: workspace_id,
	};

	update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		vec![existing.clone(), added.clone()],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));
	assert_eq!(
		BTreeSet::from([existing.clone(), added.clone()]),
		bindings_of(&setup, &admin.access_token, workspace_id, member.user_id).await,
		"an untouched binding should survive an update by someone who couldn't grant it"
	);

	// Narrowing it is still a new binding, at a scope the caller holds
	// nothing at.
	let refused = update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		vec![
			RoleBindingGrant {
				role_id: not_held.id,
				resource_id: secret.id,
			},
			added.clone(),
		],
	)
	.await;
	assert_exceeds(
		&refused,
		"a binding the caller couldn't grant should not be re-scoped by them",
	);
	assert_eq!(
		BTreeSet::from([existing.clone(), added.clone()]),
		bindings_of(&setup, &admin.access_token, workspace_id, member.user_id).await
	);

	update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		vec![added.clone()],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));

	let refused = update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		vec![existing, added.clone()],
	)
	.await;
	assert_exceeds(
		&refused,
		"a binding the caller removed should not be theirs to put back",
	);
	assert_eq!(
		BTreeSet::from([added]),
		bindings_of(&setup, &admin.access_token, workspace_id, member.user_id).await
	);
}

/// A role that holds no permissions grants nothing, so anyone who may edit
/// bindings can hand it out.
#[tokio::test]
async fn member_update_can_grant_a_role_holding_nothing() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let empty = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	// The API refuses to create a role without permissions.
	setup
		.execute_sql(&format!(
			"DELETE FROM role_permission WHERE role_id = '{}';",
			empty.id
		))
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace_id, manage.id)
		.await;

	update_member(
		&setup,
		&manager.access_token,
		workspace_id,
		member.user_id,
		grants(empty.id, &[workspace_id]),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));
	assert_eq!(
		BTreeSet::from_iter(grants(empty.id, &[workspace_id])),
		bindings_of(&setup, &admin.access_token, workspace_id, member.user_id).await
	);
}

/// A role from another workspace is refused as a role that doesn't exist,
/// whatever it holds: it's never weighed against the caller's permissions.
#[tokio::test]
async fn role_from_another_workspace_does_not_exist() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let elsewhere = setup.create_test_workspace(&manager.access_token).await;
	let foreign = setup
		.create_role_with_permissions(
			&manager.access_token,
			elsewhere.id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace_id, manage.id)
		.await;

	for response in [
		update_member(
			&setup,
			&manager.access_token,
			workspace_id,
			member.user_id,
			grants(foreign.id, &[workspace_id]),
		)
		.await,
		invite(
			&setup,
			&manager.access_token,
			workspace_id,
			grants(foreign.id, &[workspace_id]),
		)
		.await,
	] {
		assert_eq!(
			ErrorType::RoleDoesNotExist,
			response.json::<ApiErrorResponseBody>().error
		);
	}
}

/// An API token grants what it is scoped to, not what its owner could.
#[tokio::test]
async fn member_grant_is_capped_by_the_api_token_not_its_owner() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, _manager) = workspace_with_role_manager(&setup).await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace_id, manage.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&admin.access_token,
			BTreeMap::from([(
				workspace_id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([
						(
							setup.get_permission_id(Permission::ModifyRoles),
							BTreeSet::from([workspace_id]),
						),
						(
							setup.get_permission_id(Permission::ViewRoles),
							BTreeSet::from([workspace_id]),
						),
					]),
				},
			)]),
		)
		.await;
	let api_token = BearerToken::from_str(&api_token.token).unwrap();

	let update = |roles: Vec<RoleBindingGrant>| {
		setup.make_api_call(
			ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
				.path(UpdateUserRolesInWorkspacePath {
					workspace_id,
					user_id: member.user_id,
				})
				.headers(UpdateUserRolesInWorkspaceRequestHeaders {
					authorization: api_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateUserRolesInWorkspaceRequest { roles })
				.build(),
		)
	};

	let refused = update(grants(not_held.id, &[workspace_id])).await;
	assert_exceeds(
		&refused,
		"an API token should not grant what it isn't scoped to itself",
	);

	update(grants(manage.id, &[workspace_id]))
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			UpdateUserRolesInWorkspaceResponse,
		));
}

/// An invite can't carry a role the inviter doesn't hold, and a refused
/// invite leaves nothing pending.
#[tokio::test]
async fn invite_cannot_grant_a_role_the_caller_lacks() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let secret = setup
		.create_test_secret(&admin.access_token, workspace_id)
		.await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let view = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::Edit))],
		)
		.await;
	update_member(
		&setup,
		&admin.access_token,
		workspace_id,
		manager.user_id,
		vec![
			RoleBindingGrant {
				role_id: manage.id,
				resource_id: workspace_id,
			},
			RoleBindingGrant {
				role_id: view.id,
				resource_id: secret.id,
			},
		],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateUserRolesInWorkspaceResponse,
	));

	let refused = invite(
		&setup,
		&manager.access_token,
		workspace_id,
		grants(not_held.id, &[workspace_id]),
	)
	.await;
	assert_exceeds(
		&refused,
		"a role the caller doesn't hold should not be invitable",
	);

	let refused = invite(
		&setup,
		&manager.access_token,
		workspace_id,
		grants(view.id, &[workspace_id]),
	)
	.await;
	assert_exceeds(
		&refused,
		"a role held on one resource should not be invitable across the workspace",
	);

	let pending = setup
		.make_web_dashboard_call(
			ApiRequest::<ListWorkspaceInvitesRequest>::builder()
				.path(ListWorkspaceInvitesPath { workspace_id })
				.headers(ListWorkspaceInvitesRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListWorkspaceInvitesResponse>>();
	assert!(
		pending.response.invites.is_empty(),
		"a refused invite should leave nothing pending"
	);

	let allowed = invite(
		&setup,
		&manager.access_token,
		workspace_id,
		vec![
			RoleBindingGrant {
				role_id: manage.id,
				resource_id: workspace_id,
			},
			RoleBindingGrant {
				role_id: view.id,
				resource_id: secret.id,
			},
		],
	)
	.await;
	assert!(
		allowed.status_code().is_success(),
		"roles the caller holds should be invitable at the scopes they hold them, got {}",
		allowed.status_code()
	);

	let allowed = invite(
		&setup,
		&admin.access_token,
		workspace_id,
		grants(not_held.id, &[workspace_id]),
	)
	.await;
	assert!(
		allowed.status_code().is_success(),
		"the super admin should invite with any role, got {}",
		allowed.status_code()
	);
}

/// A grant already on an invite survives an edit by someone who couldn't
/// make it, and can be dropped, but can't be re-scoped or put back.
#[tokio::test]
async fn invite_update_checks_only_added_grants() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let secret = setup
		.create_test_secret(&admin.access_token, workspace_id)
		.await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let existing = RoleBindingGrant {
		role_id: not_held.id,
		resource_id: workspace_id,
	};
	let added = RoleBindingGrant {
		role_id: manage.id,
		resource_id: workspace_id,
	};
	let invite_id = invite(
		&setup,
		&admin.access_token,
		workspace_id,
		vec![existing.clone()],
	)
	.await
	.json::<ApiSuccessResponseBody<InviteUserToWorkspaceResponse>>()
	.response
	.id
	.id;

	update_invite(
		&setup,
		&manager.access_token,
		workspace_id,
		invite_id,
		vec![existing.clone(), added.clone()],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateWorkspaceInviteRolesResponse,
	));
	assert_eq!(
		BTreeSet::from([existing.clone(), added.clone()]),
		grants_of_invite(&setup, &admin.access_token, workspace_id, invite_id).await,
		"an untouched grant should survive an edit by someone who couldn't make it"
	);

	let refused = update_invite(
		&setup,
		&manager.access_token,
		workspace_id,
		invite_id,
		vec![
			RoleBindingGrant {
				role_id: not_held.id,
				resource_id: secret.id,
			},
			added.clone(),
		],
	)
	.await;
	assert_exceeds(
		&refused,
		"a grant the caller couldn't make should not be re-scoped by them",
	);
	assert_eq!(
		BTreeSet::from([existing.clone(), added.clone()]),
		grants_of_invite(&setup, &admin.access_token, workspace_id, invite_id).await,
		"a refused edit should leave the invite as it was"
	);

	update_invite(
		&setup,
		&manager.access_token,
		workspace_id,
		invite_id,
		vec![added.clone()],
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(
		UpdateWorkspaceInviteRolesResponse,
	));

	let refused = update_invite(
		&setup,
		&manager.access_token,
		workspace_id,
		invite_id,
		vec![existing, added.clone()],
	)
	.await;
	assert_exceeds(
		&refused,
		"a grant the caller removed should not be theirs to put back",
	);
	assert_eq!(
		BTreeSet::from([added]),
		grants_of_invite(&setup, &admin.access_token, workspace_id, invite_id).await
	);
}

/// Resending an invite makes it the caller's own, so they have to hold what
/// it grants.
#[tokio::test]
async fn invite_resend_cannot_reissue_a_grant_the_caller_lacks() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, workspace_id, manage, manager) = workspace_with_role_manager(&setup).await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace_id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let mut invite_ids = Vec::new();
	for role_id in [not_held.id, manage.id] {
		invite_ids.push(
			invite(
				&setup,
				&admin.access_token,
				workspace_id,
				grants(role_id, &[workspace_id]),
			)
			.await
			.json::<ApiSuccessResponseBody<InviteUserToWorkspaceResponse>>()
			.response
			.id
			.id,
		);
	}

	let refused = resend_invite(&setup, &manager.access_token, workspace_id, invite_ids[0]).await;
	assert_exceeds(
		&refused,
		"an invite granting more than the caller holds should not be theirs to resend",
	);

	for (token, invite_id) in [
		(&manager.access_token, invite_ids[1]),
		(&admin.access_token, invite_ids[0]),
	] {
		let allowed = resend_invite(&setup, token, workspace_id, invite_id).await;
		assert!(
			allowed.status_code().is_success(),
			"an invite the caller covers should be resendable, got {}",
			allowed.status_code()
		);
	}
}

/// Each family of default roles is a ladder: every tier holds everything the
/// tier below it does.
#[tokio::test]
async fn default_roles_nest_within_each_family() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let roles = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllRolesRequest>::builder()
				.path(ListAllRolesPath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.headers(ListAllRolesRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllRolesResponse>>()
		.response
		.roles;

	// Family -> tier -> the permissions that tier holds.
	let mut families = BTreeMap::<String, BTreeMap<usize, BTreeSet<Uuid>>>::new();
	for role in roles {
		assert!(
			role.data.is_immutable,
			"a new workspace should hold only default roles"
		);
		let (family, tier) =
			role.data.name.split_once(": ").unwrap_or_else(|| {
				panic!("`{}` should be named `<family>: <tier>`", role.data.name)
			});
		let tier = ["Viewer", "Pull", "Editor", "Admin"]
			.iter()
			.position(|known| *known == tier)
			.unwrap_or_else(|| panic!("`{}` has an unknown tier", role.data.name));
		families.entry(family.to_string()).or_default().insert(
			tier,
			permissions_of(&setup, &admin.access_token, workspace.id, role.id).await,
		);
	}

	assert!(
		families.contains_key("Service Account"),
		"the service account roles should be among the default roles"
	);
	for (family, tiers) in families {
		for tier in ["Viewer", "Editor", "Admin"] {
			assert!(
				tiers
					.keys()
					.any(|known| ["Viewer", "Pull", "Editor", "Admin"][*known] == tier),
				"`{family}` should have a `{tier}` role"
			);
		}
		// The map iterates from the lowest tier up.
		for (lower, higher) in tiers.values().zip(tiers.values().skip(1)) {
			assert!(
				lower.is_subset(higher),
				"each `{family}` role should hold everything the tier below it does"
			);
		}
	}
}
