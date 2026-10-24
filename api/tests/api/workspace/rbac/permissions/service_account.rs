use std::collections::BTreeSet;

use models::{
	ApiSuccessResponseBody,
	api::workspace::{
		rbac::{role::*, user::RoleBindingGrant},
		service_account::*,
	},
	rbac::{Permission, ServiceAccountPermission},
	utils::ListResourceQuery,
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
