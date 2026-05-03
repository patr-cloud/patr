use std::collections::BTreeMap;

use models::{
	api::workspace::service_account::*,
	rbac::{Permission, ServiceAccountPermission},
};

use super::{all, exclude, include, setup_permission_test};
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
		.make_api_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: ws_id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					name: random_name(8),
					description: None,
					roles: vec![],
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

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
		include(&[sa1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_api_call(
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
		.make_api_call(
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
async fn service_account_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa1 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;
	let sa2 = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
		exclude(&[sa2.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_api_call(
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
	assert!(r1.status_code().is_success(), "sa1 should be accessible");

	let r2 = setup
		.make_api_call(
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
	assert!(r2.status_code().is_client_error(), "sa2 should be excluded");
}

#[tokio::test]
async fn service_account_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let sa = setup
		.create_test_service_account(&admin.access_token, workspace.id, vec![])
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::View)),
		include(&[sa.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r_view = setup
		.make_api_call(
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
		.make_api_call(
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

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ServiceAccount(ServiceAccountPermission::Edit)),
		include(&[sa.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r_regen = setup
		.make_api_call(
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
