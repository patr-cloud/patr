use std::collections::{BTreeMap, BTreeSet};

use models::{
	ApiErrorResponseBody,
	ApiSuccessResponseBody,
	api::workspace::{
		rbac::{role::*, user::RoleBindingGrant},
		service_account::*,
	},
	rbac::{Permission, ServiceAccountPermission, WorkspacePermission},
	utils::{ListResourceQuery, Uuid},
};

use super::{all, grants, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn service_account_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(
			Permission::ServiceAccount(ServiceAccountPermission::Create),
			all(),
		)],
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: ws_id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn service_account_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[workspace.id]),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without serviceAccount::view should be denied"
	);
}

#[tokio::test]
async fn service_account_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa1 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let sa2 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
			],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[sa1.id]),
		)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa1.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success());

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa2.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r2.status_code().is_client_error());
}

#[tokio::test]
async fn service_account_scoped_grant_reaches_only_that_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa1 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let sa2 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	// The binding is scoped to sa1, so the role reaches sa1 and nothing else.
	// There is no "everything except" any more — a grant names what it covers.
	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
			],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[sa1.id]),
		)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa1.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "sa1 is in scope");

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa2.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r2.status_code().is_client_error(), "sa2 is out of scope");
}

#[tokio::test]
async fn service_account_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
			],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[sa.id]),
		)
		.await;

	let r_view = setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_view.status_code().is_success());

	let r_delete = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteServiceAccountRequest>::builder()
				.path(DeleteServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(DeleteServiceAccountRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r_delete.status_code().is_client_error(),
		"view permission should not grant delete"
	);
}

#[tokio::test]
async fn service_account_edit_does_not_grant_regenerate_token() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::Edit)),
			],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[sa.id]),
		)
		.await;

	let r_regen = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r_regen.status_code().is_client_error(),
		"edit permission should not grant regenerate token"
	);
}

/// A member with no service account permission gets a membership-gated list
/// that succeeds but is View-filtered to empty — not a 403.
#[tokio::test]
async fn service_account_no_permission_list_returns_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;
	let _sa = setup
		.create_test_service_account(&admin.access_token, ws_id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: ws_id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();
	assert!(
		response.response.service_accounts.is_empty(),
		"a member without service account View should see an empty list, not a 403"
	);
}

/// The total count only covers the service accounts the member can view.
#[tokio::test]
async fn service_account_view_include_list_counts_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa1 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
			],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[sa1.id]),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!("1", response.header("x-total-count"));
	let body = response.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();
	assert_eq!(1, body.response.service_accounts.len());
	assert_eq!(sa1.id, body.response.service_accounts[0].id);
}

/// Changing a service account's bindings reaches its token on the next
/// request, even once the token's permissions are cached.
#[tokio::test]
async fn service_account_rebinding_takes_effect_at_once() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let sa_token = BearerToken::from_str(&sa.token).unwrap();
	let view_roles = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
		.await;

	let get_role_info = || {
		setup.make_api_call(
			ApiRequest::<GetRoleInfoRequest>::builder()
				.path(GetRoleInfoPath {
					workspace_id: workspace.id,
					role_id: view_roles.id,
				})
				.headers(GetRoleInfoRequestHeaders {
					authorization: sa_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};
	let rebind = |role_bindings| {
		setup.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: sa.name.clone(),
						description: None,
					},
					role_bindings,
				})
				.build(),
		)
	};

	// Unbound, and now cached as such.
	assert_eq!(401, get_role_info().await.status_code().as_u16());

	rebind(vec![RoleBindingGrant {
		role_id: view_roles.id,
		resource_id: workspace.id,
	}])
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	assert!(
		get_role_info().await.status_code().is_success(),
		"a new binding should reach the cached token at once"
	);

	rebind(vec![])
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	assert_eq!(
		401,
		get_role_info().await.status_code().as_u16(),
		"a removed binding should drop out of the cached token at once"
	);
}

/// A new workspace is seeded with the three service account tiers, each
/// immutable and holding exactly its tier's permissions.
#[tokio::test]
async fn service_account_default_roles_are_seeded() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

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
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllRolesResponse>>()
		.response
		.roles;

	let viewer = vec![ServiceAccountPermission::View];
	let editor = vec![
		ServiceAccountPermission::View,
		ServiceAccountPermission::Create,
		ServiceAccountPermission::Edit,
		ServiceAccountPermission::RegenerateToken,
	];
	let admin = vec![
		ServiceAccountPermission::View,
		ServiceAccountPermission::Create,
		ServiceAccountPermission::Edit,
		ServiceAccountPermission::RegenerateToken,
		ServiceAccountPermission::Delete,
	];
	for (name, permissions) in [
		("Service Account: Viewer", viewer),
		("Service Account: Editor", editor),
		("Service Account: Admin", admin),
	] {
		let role = roles
			.iter()
			.find(|role| role.data.name == name)
			.unwrap_or_else(|| panic!("a new workspace should seed `{name}`"));
		assert!(role.data.is_immutable, "`{name}` should be immutable");

		let info = setup
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
			permissions
				.into_iter()
				.map(|permission| setup.get_permission_id(Permission::ServiceAccount(permission)))
				.collect::<BTreeSet<_>>(),
			info.response.permissions,
			"`{name}` should hold exactly its tier's permissions"
		);
	}
}

/// Create a service account named at random with `role_bindings`, as `token`.
async fn create_with_bindings(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	role_bindings: Vec<RoleBindingGrant>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath { workspace_id })
				.headers(CreateServiceAccountRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
					},
					role_bindings,
				})
				.build(),
		)
		.await
}

/// Replace the bindings of `service_account` with `role_bindings`, as `token`.
async fn update_bindings(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	service_account: &TestServiceAccount,
	role_bindings: Vec<RoleBindingGrant>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id,
					service_account_id: service_account.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: service_account.name.clone(),
						description: None,
					},
					role_bindings,
				})
				.build(),
		)
		.await
}

/// The bindings `service_account_id` holds, as `token` sees them.
async fn bindings_of(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	service_account_id: Uuid,
) -> Vec<RoleBindingGrant> {
	setup
		.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id,
					service_account_id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>()
		.response
		.role_bindings
}

/// Creating a service account can't hand it a role the creator doesn't hold,
/// and the refused create leaves nothing behind.
#[tokio::test]
async fn service_account_create_cannot_grant_a_role_the_caller_lacks() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let held =
		setup
			.create_role_with_permissions(
				&admin.access_token,
				workspace.id,
				vec![setup.get_permission_id(Permission::ServiceAccount(
					ServiceAccountPermission::Create,
				))],
			)
			.await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ModifyRoles)],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(held.id, &[workspace.id]),
		)
		.await;

	let refused = create_with_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		grants(not_held.id, &[workspace.id]),
	)
	.await;
	assert_eq!(
		403,
		refused.status_code().as_u16(),
		"a role the caller doesn't hold should not be grantable"
	);

	// One role the caller holds doesn't carry one they don't.
	let refused = create_with_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		vec![
			RoleBindingGrant {
				role_id: held.id,
				resource_id: workspace.id,
			},
			RoleBindingGrant {
				role_id: not_held.id,
				resource_id: workspace.id,
			},
		],
	)
	.await;
	assert_eq!(403, refused.status_code().as_u16());

	let listed = setup
		.make_web_dashboard_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();
	assert!(
		listed.response.service_accounts.is_empty(),
		"a refused create should leave no service account behind"
	);

	let allowed = create_with_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		grants(held.id, &[workspace.id]),
	)
	.await;
	assert!(
		allowed.status_code().is_success(),
		"a role the caller holds should be grantable, got {}",
		allowed.status_code()
	);
}

/// Updating a service account can't hand it a role the editor doesn't hold,
/// and the refused update leaves its bindings as they were.
#[tokio::test]
async fn service_account_update_cannot_grant_a_role_the_caller_lacks() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::Edit)),
			],
		)
		.await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ModifyRoles)],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(held.id, &[workspace.id]),
		)
		.await;

	let refused = update_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		&sa,
		grants(not_held.id, &[workspace.id]),
	)
	.await;
	assert_eq!(
		403,
		refused.status_code().as_u16(),
		"a role the caller doesn't hold should not be grantable"
	);
	assert!(
		bindings_of(&setup, &admin.access_token, workspace.id, sa.id)
			.await
			.is_empty(),
		"a refused update should leave the bindings as they were"
	);

	update_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		&sa,
		grants(held.id, &[workspace.id]),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	assert_eq!(
		grants(held.id, &[workspace.id]),
		bindings_of(&setup, &admin.access_token, workspace.id, sa.id).await
	);
}

/// A role held on one resource can be passed on at that resource, and no
/// wider: not at another resource, and not across the workspace.
#[tokio::test]
async fn service_account_grant_cannot_be_wider_than_the_callers_scope() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa1 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let sa2 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let manage = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(
					ServiceAccountPermission::Create,
				)),
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::Edit)),
			],
		)
		.await;
	let view = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
			],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			vec![
				RoleBindingGrant {
					role_id: manage.id,
					resource_id: workspace.id,
				},
				RoleBindingGrant {
					role_id: view.id,
					resource_id: sa1.id,
				},
			],
		)
		.await;

	for scope in [workspace.id, sa2.id] {
		let refused = create_with_bindings(
			&setup,
			&user_b.access_token,
			workspace.id,
			grants(view.id, &[scope]),
		)
		.await;
		assert_eq!(
			403,
			refused.status_code().as_u16(),
			"a role held on one resource should not be grantable at `{scope}`"
		);
	}

	let allowed = create_with_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		grants(view.id, &[sa1.id]),
	)
	.await;
	assert!(
		allowed.status_code().is_success(),
		"a role should be grantable at the scope the caller holds it at, got {}",
		allowed.status_code()
	);

	// The same on update: the grant made at sa1 can't be moved to the root.
	update_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		&sa2,
		grants(view.id, &[sa1.id]),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	for scope in [workspace.id, sa2.id] {
		let refused = update_bindings(
			&setup,
			&user_b.access_token,
			workspace.id,
			&sa2,
			grants(view.id, &[scope]),
		)
		.await;
		assert_eq!(
			403,
			refused.status_code().as_u16(),
			"a role held on one resource should not be re-scoped to `{scope}`"
		);
	}
	assert_eq!(
		grants(view.id, &[sa1.id]),
		bindings_of(&setup, &admin.access_token, workspace.id, sa2.id).await
	);
}

/// A binding the editor couldn't make themselves survives their update as
/// long as it's sent back untouched. They can drop it, but not re-scope it.
#[tokio::test]
async fn service_account_update_keeps_a_grant_the_caller_could_not_make() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let other = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let edit = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::Edit)),
			],
		)
		.await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ModifyRoles)],
		)
		.await;
	let existing = grants(not_held.id, &[other.id]);
	let sa = setup
		.create_test_service_account(&admin.access_token, workspace.id, existing.clone())
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(edit.id, &[workspace.id]),
		)
		.await;

	update_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		&sa,
		existing.clone(),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	assert_eq!(
		existing,
		bindings_of(&setup, &admin.access_token, workspace.id, sa.id).await,
		"an untouched binding should survive an update by someone who couldn't grant it"
	);

	let refused = update_bindings(
		&setup,
		&user_b.access_token,
		workspace.id,
		&sa,
		grants(not_held.id, &[workspace.id]),
	)
	.await;
	assert_eq!(
		403,
		refused.status_code().as_u16(),
		"a binding the caller couldn't grant should not be re-scoped by them"
	);
	assert_eq!(
		existing,
		bindings_of(&setup, &admin.access_token, workspace.id, sa.id).await
	);

	update_bindings(&setup, &user_b.access_token, workspace.id, &sa, vec![])
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	assert!(
		bindings_of(&setup, &admin.access_token, workspace.id, sa.id)
			.await
			.is_empty(),
		"a binding the caller couldn't grant should still be removable"
	);
}

/// The ceiling is what the credential in hand can do, not what its owner can:
/// the super admin grants anything, but their API token only what it's scoped
/// to.
#[tokio::test]
async fn service_account_grant_is_capped_by_the_api_token_not_its_owner() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ModifyRoles)],
		)
		.await;
	let create =
		setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::Create));
	let edit = setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::Edit));
	let api_token = setup
		.create_test_api_token(
			&admin.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([
						(create, BTreeSet::from([workspace.id])),
						(edit, BTreeSet::from([workspace.id])),
					]),
				},
			)]),
		)
		.await;
	let api_token = BearerToken::from_str(&api_token.token).unwrap();

	let sa = setup
		.create_test_service_account(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[workspace.id]),
		)
		.await;
	update_bindings(&setup, &admin.access_token, workspace.id, &sa, vec![])
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	update_bindings(
		&setup,
		&admin.access_token,
		workspace.id,
		&sa,
		grants(role.id, &[workspace.id]),
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));
	update_bindings(&setup, &admin.access_token, workspace.id, &sa, vec![])
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));

	let refused = setup
		.make_api_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: api_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
					},
					role_bindings: grants(role.id, &[workspace.id]),
				})
				.build(),
		)
		.await;
	assert_eq!(
		403,
		refused.status_code().as_u16(),
		"an API token should not grant what it isn't scoped to itself"
	);

	let refused = setup
		.make_api_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: api_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: sa.name.clone(),
						description: None,
					},
					role_bindings: grants(role.id, &[workspace.id]),
				})
				.build(),
		)
		.await;
	assert_eq!(403, refused.status_code().as_u16());
	assert!(
		bindings_of(&setup, &admin.access_token, workspace.id, sa.id)
			.await
			.is_empty()
	);

	let allowed = setup
		.make_api_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: api_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;
	assert!(
		allowed.status_code().is_success(),
		"the token should still create a service account it grants nothing to, got {}",
		allowed.status_code()
	);
}

/// A regenerated token is a working credential for the account, so it's only
/// handed to someone who holds everything the account does.
#[tokio::test]
async fn service_account_token_cannot_be_regenerated_past_the_callers_permissions() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let regenerate = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ServiceAccount(
				ServiceAccountPermission::RegenerateToken,
			))],
		)
		.await;
	let not_held = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ModifyRoles)],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(regenerate.id, &[workspace.id]),
		)
		.await;
	let covered = setup
		.create_test_service_account(
			&admin.access_token,
			workspace.id,
			grants(regenerate.id, &[workspace.id]),
		)
		.await;
	let exceeding = setup
		.create_test_service_account(
			&admin.access_token,
			workspace.id,
			vec![
				RoleBindingGrant {
					role_id: regenerate.id,
					resource_id: workspace.id,
				},
				RoleBindingGrant {
					role_id: not_held.id,
					resource_id: workspace.id,
				},
			],
		)
		.await;

	let regenerate_as = |token: &BearerToken, service_account_id: Uuid| {
		setup.make_web_dashboard_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	let refused = regenerate_as(&user_b.access_token, exceeding.id).await;
	assert_eq!(
		403,
		refused.status_code().as_u16(),
		"a token for an account holding more than the caller should not be handed out"
	);
	assert_eq!(
		ErrorType::GrantExceedsPermissions,
		refused.json::<ApiErrorResponseBody>().error
	);

	// The refused call left the account's token as it was: it still passes
	// for the account, which holds `ModifyRoles`.
	let still_valid = setup
		.make_api_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath {
					workspace_id: workspace.id,
				})
				.headers(CreateNewRoleRequestHeaders {
					authorization: BearerToken::from_str(&exceeding.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: "test".to_string(),
						is_immutable: false,
					},
					permissions: BTreeSet::from([setup.get_permission_id(Permission::ModifyRoles)]),
				})
				.build(),
		)
		.await;
	assert!(
		still_valid.status_code().is_success(),
		"a refused regenerate should not revoke the account's token, got {}",
		still_valid.status_code()
	);

	for (token, service_account_id) in [
		(&user_b.access_token, covered.id),
		(&admin.access_token, exceeding.id),
	] {
		let allowed = regenerate_as(token, service_account_id).await;
		assert!(
			allowed.status_code().is_success(),
			"a token for an account the caller covers should be handed out, got {}",
			allowed.status_code()
		);
	}
}
