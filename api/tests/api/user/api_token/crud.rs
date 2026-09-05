//! Minting, listing, reading, renaming, revoking and regenerating a token.

use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use models::{
	ApiSuccessResponseBody,
	api::user::*,
	rbac::WorkspacePermission,
	utils::{ListResourceQuery, Uuid},
};

use crate::prelude::*;

/// A token name frees up once the token is revoked and can be reused.
#[tokio::test]
async fn api_token_name_reusable_after_revoke() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let name = random_name(8);
	let super_admins = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

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
						permissions: super_admins.clone(),
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
						permissions: super_admins.clone(),
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
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	assert!(!api_token.token.is_empty(), "token should not be empty");
}

#[tokio::test]
async fn list_api_tokens_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let super_admins = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	let _t1 = setup
		.create_test_api_token(&user.access_token, super_admins.clone())
		.await;
	let _t2 = setup
		.create_test_api_token(&user.access_token, super_admins)
		.await;

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
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
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
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
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
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::SuperAdmin,
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
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
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
	let super_admins = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	let first = setup
		.create_test_api_token(&user.access_token, super_admins.clone())
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
						permissions: super_admins.clone(),
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
	let super_admins = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	let first = setup
		.create_test_api_token(&user.access_token, super_admins.clone())
		.await;
	let second = setup
		.create_test_api_token(&user.access_token, super_admins)
		.await;

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
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::SuperAdmin,
						)]),
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
async fn list_api_tokens_pagination() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let super_admins = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	for _ in 0..3 {
		setup
			.create_test_api_token(&user.access_token, super_admins.clone())
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
