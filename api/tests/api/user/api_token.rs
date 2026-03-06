use std::collections::BTreeMap;

use models::{ApiSuccessResponseBody, api::user::*, utils::Uuid};

use crate::prelude::*;

#[tokio::test]
async fn create_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;
	assert!(!api_token.token.is_empty(), "token should not be empty");
}

#[tokio::test]
async fn list_api_tokens_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let _t1 = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;
	let _t2 = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;

	let response = setup
		.make_api_call(
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
	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;

	let response = setup
		.make_api_call(
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
	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;
	let new_name = random_name(8);

	setup
		.make_api_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: api_token.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					name: Some(new_name.clone()),
					permissions: None,
					token_nbf: None,
					token_exp: None,
					allowed_ips: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateApiTokenResponse));

	// Verify the update
	let response = setup
		.make_api_call(
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
	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;

	setup
		.make_api_call(
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
		.make_api_call(
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
	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeMap::new())
		.await;

	let response = setup
		.make_api_call(
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
		.make_api_call(
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

/// Bug: creating an API token with empty permissions should be rejected,
/// but the endpoint currently accepts it silently. This test asserts the
/// correct behavior and is expected to FAIL until the bug is fixed.
#[tokio::test]
async fn create_api_token_with_empty_permissions_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_api_call(
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
		.make_api_call(
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
