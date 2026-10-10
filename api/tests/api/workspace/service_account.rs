use api::redis::keys;
use models::{
	ApiSuccessResponseBody,
	api::workspace::{
		GetWorkspaceInfoPath,
		GetWorkspaceInfoRequest,
		GetWorkspaceInfoRequestHeaders,
		rbac::user::RoleBindingGrant,
		service_account::*,
	},
	rbac::Permission,
	utils::Uuid,
};
use sha2::{Digest as _, Sha256};

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
		sa.token.starts_with("patr_sa_"),
		"token should start with patr_sa_"
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
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: sa.name,
						description: None,
						is_immutable: false,
					},
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
		.make_web_dashboard_call(
			ApiRequest::<CreateServiceAccountRequest>::builder()
				.path(CreateServiceAccountPath {
					workspace_id: workspace.id,
				})
				.headers(CreateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateServiceAccountRequest {
					service_account: ServiceAccount {
						name: "!!!".to_string(),
						description: None,
						is_immutable: false,
					},
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
					service_account: ServiceAccount {
						name: new_name.clone(),
						description: None,
						is_immutable: false,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));

	// Verify
	let response = setup
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
					service_account: ServiceAccount {
						name: sa.name.clone(),
						description: None,
						is_immutable: false,
					},
					role_bindings: vec![grant.clone()],
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateServiceAccountResponse));

	// Verify the grants came back
	let response = setup
		.make_web_dashboard_call(
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

	assert_eq!(vec![grant], response.response.role_bindings);
}

#[tokio::test]
async fn update_service_account_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let taken = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;

	let response = setup
		.make_web_dashboard_call(
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
					service_account: ServiceAccount {
						name: taken.name,
						description: None,
						is_immutable: false,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn update_service_account_of_another_type_is_not_found() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id: runner.id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
						is_immutable: false,
					},
					role_bindings: vec![],
				})
				.build(),
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		new_token.starts_with("patr_sa_"),
		"new token should start with patr_sa_"
	);
	assert_ne!(
		&sa.token, new_token,
		"new token should differ from original"
	);
}

/// A regenerated service account's old token stops working at once, even
/// once cached, and its cached entry is dropped.
#[tokio::test]
async fn regenerated_service_account_old_token_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let get_workspace_info = |token: &str| {
		setup.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	assert!(
		get_workspace_info(&sa.token)
			.await
			.status_code()
			.is_success(),
		"the token should work before it is regenerated"
	);
	let cache_key = keys::auth_data_for_token(&hex::encode(Sha256::digest(&sa.token)));
	assert!(
		setup.get_redis_value(&cache_key).await.is_some(),
		"the token should be cached"
	);

	let new_token = setup
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<RegenerateServiceAccountTokenResponse>>()
		.response
		.token;

	assert!(
		setup.get_redis_value(&cache_key).await.is_none(),
		"regenerating should drop the old token's cached entry"
	);
	assert_eq!(
		401,
		get_workspace_info(&sa.token).await.status_code().as_u16(),
		"the old token should be rejected with 401"
	);
	assert!(
		get_workspace_info(&new_token)
			.await
			.status_code()
			.is_success(),
		"the regenerated token should work"
	);
}

/// A deleted service account's token stops working at once, even once cached,
/// and its cached entry is dropped.
#[tokio::test]
async fn deleted_service_account_token_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let get_workspace_info = || {
		setup.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(&sa.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	assert!(
		get_workspace_info().await.status_code().is_success(),
		"the token should work before the account is deleted"
	);
	let cache_key = keys::auth_data_for_token(&hex::encode(Sha256::digest(&sa.token)));
	assert!(
		setup.get_redis_value(&cache_key).await.is_some(),
		"the token should be cached"
	);

	assert!(
		setup
			.make_web_dashboard_call(
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
			.status_code()
			.is_success(),
		"deleting the account should succeed"
	);

	assert!(
		setup.get_redis_value(&cache_key).await.is_none(),
		"deleting should drop the cached entry"
	);
	assert_eq!(
		401,
		get_workspace_info().await.status_code().as_u16(),
		"a deleted account's token should be rejected with 401"
	);
}

// ── Runner service accounts ─────────────────────────────────────────────

/// A runner's service account is listed and readable like any other, flagged
/// immutable. One made on the service account routes isn't.
#[tokio::test]
async fn runner_service_account_is_immutable() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let sa = setup
		.create_test_service_account(&user.access_token, workspace.id, vec![])
		.await;
	let get_service_account_info = |service_account_id| {
		setup.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	let service_accounts = setup
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>()
		.response
		.service_accounts;
	assert_eq!(2, service_accounts.len());
	let runner_sa = service_accounts
		.iter()
		.find(|listed| listed.name == format!("runner-{}", runner.id))
		.expect("the runner's service account should be listed");
	assert!(runner_sa.is_immutable);
	assert!(
		!service_accounts
			.iter()
			.find(|listed| listed.id == sa.id)
			.expect("the service account should be listed")
			.is_immutable
	);

	assert!(
		get_service_account_info(runner_sa.id)
			.await
			.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>()
			.response
			.service_account
			.is_immutable
	);
	assert!(
		!get_service_account_info(sa.id)
			.await
			.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>()
			.response
			.service_account
			.is_immutable
	);
}

/// A runner's service account can't be updated, have its token regenerated or
/// be deleted on the service account routes, and none of them change it.
#[tokio::test]
async fn runner_service_account_cannot_be_changed() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let role = setup
		.create_role_with_permissions(
			&user.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
		.await;
	let service_account_id = setup
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<ListServiceAccountsResponse>>()
		.response
		.service_accounts[0]
		.id;
	let get_service_account_info = || {
		setup.make_web_dashboard_call(
			ApiRequest::<GetServiceAccountInfoRequest>::builder()
				.path(GetServiceAccountInfoPath {
					workspace_id: workspace.id,
					service_account_id,
				})
				.headers(GetServiceAccountInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};
	let before = get_service_account_info()
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>()
		.response;

	let update = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateServiceAccountRequest>::builder()
				.path(UpdateServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id,
				})
				.headers(UpdateServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateServiceAccountRequest {
					service_account: ServiceAccount {
						name: random_name(8),
						description: None,
						is_immutable: false,
					},
					role_bindings: vec![RoleBindingGrant {
						role_id: role.id,
						resource_id: workspace.id,
					}],
				})
				.build(),
		)
		.await;
	assert_eq!(StatusCode::FORBIDDEN, update.status_code());

	let regenerate = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateServiceAccountTokenRequest>::builder()
				.path(RegenerateServiceAccountTokenPath {
					workspace_id: workspace.id,
					service_account_id,
				})
				.headers(RegenerateServiceAccountTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(StatusCode::FORBIDDEN, regenerate.status_code());

	let delete = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteServiceAccountRequest>::builder()
				.path(DeleteServiceAccountPath {
					workspace_id: workspace.id,
					service_account_id,
				})
				.headers(DeleteServiceAccountRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(StatusCode::FORBIDDEN, delete.status_code());

	let after = get_service_account_info()
		.await
		.json::<ApiSuccessResponseBody<GetServiceAccountInfoResponse>>()
		.response;
	assert_eq!(before.service_account, after.service_account);
	assert_eq!(before.role_bindings.len(), after.role_bindings.len());
	assert!(
		before
			.role_bindings
			.iter()
			.all(|grant| after.role_bindings.contains(grant))
	);

	setup
		.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();
}

// ── Unauthorized ────────────────────────────────────────────────────────

#[tokio::test]
async fn service_account_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
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
