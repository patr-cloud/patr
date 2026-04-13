use models::{
	ApiSuccessResponseBody,
	api::workspace::{rbac::user::RoleBindingGrant, service_account::*},
	rbac::Permission,
	utils::Uuid,
};

use crate::prelude::*;

// ── Create ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_service_account_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	assert!(!sa.name.is_empty());
	assert!(
		sa.token.starts_with("patrv1."),
		"token should start with patrv1."
	);
}

#[tokio::test]
async fn create_service_account_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					name: sa.name,
					description: None,
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate service account name"
	);
}

#[tokio::test]
async fn create_service_account_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					name: "!!!".to_string(),
					description: None,
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid service account name"
	);
}

// ── List ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_service_accounts_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();

	assert_eq!(1, response.response.service_accounts.len());
}

#[tokio::test]
async fn list_service_accounts_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>();

	assert!(response.response.service_accounts.is_empty());
}

// ── Get ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_service_account_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>();

	assert_eq!(sa.name, response.response.service_account.name);
}

#[tokio::test]
async fn get_service_account_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: Uuid::nil(),
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent service account"
	);
}

// ── Update ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_service_account_name_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let new_name = random_name(8);
	setup
		.make_api_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					name: Some(new_name.clone()),
					description: None,
					role_bindings: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));

	// Verify
	let response = setup
		.make_api_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>();

	assert_eq!(new_name, response.response.service_account.name);
}

#[tokio::test]
async fn update_service_account_role_bindings_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let role = setup
		.create_role_with_permissions(
			&user.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
		.await;
	// The whole workspace: a grant at the root covers everything under it.
	let grant = RoleBindingGrant {
		role_id: role.id,
		resource_id: workspace.id,
	};

	setup
		.make_api_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					name: None,
					description: None,
					role_bindings: Some(vec![grant.clone()]),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));

	// Verify the grants came back
	let response = setup
		.make_api_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>();

	assert_eq!(vec![grant], response.response.service_account.role_bindings);
}

// ── Delete ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_service_account_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	setup
		.make_api_call(
			ApiRequest::<DeleteServiceAccountRequest>::builder()
				.path(DeleteServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(DeleteServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteServiceAccountResponse));

	// Verify it's gone
	let response = setup
		.make_api_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleted service account should not be found"
	);
}

#[tokio::test]
async fn delete_service_account_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<DeleteServiceAccountRequest>::builder()
				.path(DeleteServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: Uuid::nil(),
				})
				.headers(DeleteServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent service account"
	);
}

// ── Token Regeneration ──────────────────────────────────────────────────

#[tokio::test]
async fn regenerate_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id: sa.id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RegenerateServiceAccountTokenResponse>>();

	let new_token = &response.response.token;
	assert!(
		new_token.starts_with("patrv1."),
		"new token should start with patrv1."
	);
	assert_ne!(
		&sa.token, new_token,
		"new token should differ from original"
	);
}

// NOTE: The following tests require a ClientType::ApiToken test server, which
// the test infra doesn't currently support (it only runs WebDashboard mode).
// See api/tests/TODOs.md for tracking:
// - service_account_token_authenticates
// - service_account_token_deleted_sa_fails
// - sa_without_runner_permission_denied
// - user_api_token_still_works_after_sa_feature
// - regenerate_token_invalidates_old (the invalidation check)

// ── Unauthorized ────────────────────────────────────────────────────────

#[tokio::test]
async fn service_account_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListServiceAccountsRequest>::builder()
				.path(ListServiceAccountsPath {
					workspace_id: workspace.id,
				})
				.headers(ListServiceAccountsRequestHeaders {
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
