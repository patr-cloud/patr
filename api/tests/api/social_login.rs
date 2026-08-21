use api::{
	models::social_login::{GithubSetupPayload, GithubStatePayload},
	redis::keys as redis_keys,
};
use models::{
	ApiSuccessResponseBody,
	api::{auth::*, user::*},
	utils::Uuid,
};

use crate::prelude::*;

// ---------------------------------------------------------------------------
// POST /auth/social-login/github (initiate)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn social_login_initiate_works() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginInitiateRequest>::builder()
				.path(SocialLoginInitiatePath {
					provider: SocialLoginProvider::GitHub,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<SocialLoginInitiateResponse>>();

	let url =
		reqwest::Url::parse(&response.response.authorize_url).expect("authorize_url should parse");
	assert_eq!(url.host_str(), Some("github.com"));
	assert_eq!(url.path(), "/login/oauth/authorize");

	let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
	assert!(
		pairs.contains_key("client_id"),
		"missing client_id query param"
	);
	let state_token = pairs.get("state").expect("missing state query param");

	let stored = setup
		.get_redis_value(&redis_keys::social_login_state(
			&SocialLoginProvider::GitHub,
			state_token,
		))
		.await
		.expect("state key should exist in redis");

	let payload: GithubStatePayload =
		serde_json::from_str(&stored).expect("state payload should deserialise");
	assert!(
		matches!(payload, GithubStatePayload::Anonymous),
		"initiate should store an Anonymous state payload"
	);
}

// ---------------------------------------------------------------------------
// POST /auth/social-login/github/callback (CSRF / state validation)
//
// Both tests below short-circuit before any HTTP call to GitHub — the handler
// validates the CSRF state token against Redis first.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn social_login_callback_invalid_state() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginCallbackRequest>::builder()
				.path(SocialLoginCallbackPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(SocialLoginCallbackRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(SocialLoginCallbackRequest {
					code: "irrelevant-code".to_string(),
					// Random UUID — never written to Redis, getdel returns None.
					state: Uuid::new_v4().to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for missing state, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn social_login_callback_rejects_authenticated_state() {
	let setup = setup().await.expect("failed to setup test server");

	let state_token = Uuid::new_v4().to_string();
	let payload = GithubStatePayload::Authenticated {
		user_id: Uuid::new_v4(),
	};
	setup
		.set_redis_value(
			&redis_keys::social_login_state(&SocialLoginProvider::GitHub, &state_token),
			&serde_json::to_string(&payload).unwrap(),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginCallbackRequest>::builder()
				.path(SocialLoginCallbackPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(SocialLoginCallbackRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(SocialLoginCallbackRequest {
					code: "irrelevant-code".to_string(),
					state: state_token,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"auth callback should reject a Connect-variant state token, got {}",
		response.status_code()
	);
}

// ---------------------------------------------------------------------------
// POST /auth/social-login/github/setup
// ---------------------------------------------------------------------------

/// Pre-write a valid setup payload to Redis and return the setup token.
async fn seed_github_setup(setup: &TestSetup, email: &str) -> String {
	let token = Uuid::new_v4().to_string();
	let external_id = format!("gh-{}", rand::random::<u64>());
	let payload = GithubSetupPayload {
		external_id,
		email: email.to_string(),
	};
	setup
		.set_redis_value(
			&redis_keys::social_login_setup(&SocialLoginProvider::GitHub, &token),
			&serde_json::to_string(&payload).unwrap(),
		)
		.await;
	token
}

#[tokio::test]
async fn social_login_setup_invalid_token() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginSetupRequest>::builder()
				.path(SocialLoginSetupPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(SocialLoginSetupRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(SocialLoginSetupRequest {
					setup_token: Uuid::new_v4().to_string(),
					first_name: "Test".to_string(),
					last_name: "User".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid setup token, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn social_login_setup_works() {
	let setup = setup().await.expect("failed to setup test server");

	let email = format!("{}@example.com", random_name(6));
	let setup_token = seed_github_setup(&setup, &email).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginSetupRequest>::builder()
				.path(SocialLoginSetupPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(SocialLoginSetupRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(SocialLoginSetupRequest {
					setup_token,
					first_name: "Octo".to_string(),
					last_name: "Cat".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<SocialLoginSetupResponse>>();

	assert!(
		!response.response.access_token.is_empty(),
		"setup should return an access token"
	);

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&response.response.access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(email, info.response.email);
	assert_eq!("Octo", info.response.basic_user_info.first_name);
	assert_eq!("Cat", info.response.basic_user_info.last_name);
}

#[tokio::test]
async fn social_login_setup_token_single_use() {
	let setup = setup().await.expect("failed to setup test server");

	let email = format!("{}@example.com", random_name(6));
	let setup_token = seed_github_setup(&setup, &email).await;

	// First call succeeds.
	setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginSetupRequest>::builder()
				.path(SocialLoginSetupPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(SocialLoginSetupRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(SocialLoginSetupRequest {
					setup_token: setup_token.clone(),
					first_name: "Octo".to_string(),
					last_name: "Cat".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<SocialLoginSetupResponse>>();

	// Second call with the same token must fail — getdel made it single-use.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginSetupRequest>::builder()
				.path(SocialLoginSetupPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(SocialLoginSetupRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(SocialLoginSetupRequest {
					setup_token,
					first_name: "Octo".to_string(),
					last_name: "Cat".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"setup token should be single-use, got {} on second call",
		response.status_code()
	);
}

#[tokio::test]
async fn social_login_setup_email_taken() {
	let setup = setup().await.expect("failed to setup test server");
	let existing = setup.create_test_user().await;

	let setup_token = seed_github_setup(&setup, &existing.email).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SocialLoginSetupRequest>::builder()
				.path(SocialLoginSetupPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(SocialLoginSetupRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(SocialLoginSetupRequest {
					setup_token,
					first_name: "Octo".to_string(),
					last_name: "Cat".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for taken email, got {}",
		response.status_code()
	);
}
