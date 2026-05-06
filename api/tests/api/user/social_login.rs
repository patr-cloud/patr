use api::{
	models::social_login::GithubStatePayload,
	redis::keys as redis_keys,
};
use models::{
	ApiSuccessResponseBody,
	api::{auth::SocialLoginProvider, user::*},
	utils::Uuid,
};

use crate::prelude::*;

// ---------------------------------------------------------------------------
// POST /user/social-login/{provider}/connect (initiate, authenticated)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn connect_social_login_initiate_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ConnectSocialLoginInitiateRequest>::builder()
				.path(ConnectSocialLoginInitiatePath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(ConnectSocialLoginInitiateRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ConnectSocialLoginInitiateResponse>>();

	let url = reqwest::Url::parse(&response.response.authorize_url)
		.expect("authorize_url should parse");
	let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
	assert!(pairs.contains_key("client_id"), "missing client_id query param");
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
	match payload {
		GithubStatePayload::Authenticated { user_id } => {
			assert_eq!(user_id, user.user_id, "state user_id must match the caller")
		}
		GithubStatePayload::Anonymous => {
			panic!("connect-flow state should be Authenticated, got Anonymous")
		}
	}
}

#[tokio::test]
async fn connect_social_login_initiate_unauthenticated() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ConnectSocialLoginInitiateRequest>::builder()
				.path(ConnectSocialLoginInitiatePath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(ConnectSocialLoginInitiateRequestHeaders {
					authorization: BearerToken::from_str("not-a-real-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"connect initiate without a valid token should fail, got {}",
		response.status_code()
	);
}

// ---------------------------------------------------------------------------
// POST /user/social-login/{provider}/callback (authenticated)
//
// All three tests fail before any GitHub HTTP call.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn connect_callback_invalid_state() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ConnectSocialLoginCallbackRequest>::builder()
				.path(ConnectSocialLoginCallbackPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(ConnectSocialLoginCallbackRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ConnectSocialLoginCallbackRequest {
					code: "irrelevant-code".to_string(),
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
async fn connect_callback_rejects_anonymous_state() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let state_token = Uuid::new_v4().to_string();
	setup
		.set_redis_value(
			&redis_keys::social_login_state(&SocialLoginProvider::GitHub, &state_token),
			&serde_json::to_string(&GithubStatePayload::Anonymous).unwrap(),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ConnectSocialLoginCallbackRequest>::builder()
				.path(ConnectSocialLoginCallbackPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(ConnectSocialLoginCallbackRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ConnectSocialLoginCallbackRequest {
					code: "irrelevant-code".to_string(),
					state: state_token,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"connect callback should reject an Anonymous state, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn connect_callback_user_mismatch() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let user_b = setup.create_test_user().await;

	// Mint an Authenticated state token bound to user A.
	let state_token = Uuid::new_v4().to_string();
	let payload = GithubStatePayload::Authenticated {
		user_id: user_a.user_id,
	};
	setup
		.set_redis_value(
			&redis_keys::social_login_state(&SocialLoginProvider::GitHub, &state_token),
			&serde_json::to_string(&payload).unwrap(),
		)
		.await;

	// Hit the callback as user B.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ConnectSocialLoginCallbackRequest>::builder()
				.path(ConnectSocialLoginCallbackPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(ConnectSocialLoginCallbackRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ConnectSocialLoginCallbackRequest {
					code: "irrelevant-code".to_string(),
					state: state_token,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"connect callback should reject a state token bound to a different user, got {}",
		response.status_code()
	);
}

// ---------------------------------------------------------------------------
// GET /user/social-login (list)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_social_logins_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListSocialLoginsRequest>::builder()
				.headers(ListSocialLoginsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListSocialLoginsResponse>>();

	assert!(
		response.response.logins.is_empty(),
		"a fresh user should have no linked social logins"
	);
}

#[tokio::test]
async fn list_social_logins_with_github() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.execute_sql(&format!(
			"INSERT INTO user_social_login (user_id, provider, external_id, linked_at)
			 VALUES ('{}', 'github', 'gh-{}', NOW())",
			user.user_id,
			rand::random::<u64>(),
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListSocialLoginsRequest>::builder()
				.headers(ListSocialLoginsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListSocialLoginsResponse>>();

	assert_eq!(1, response.response.logins.len());
	assert_eq!(
		SocialLoginProvider::GitHub,
		response.response.logins[0].provider
	);
}

// ---------------------------------------------------------------------------
// DELETE /user/social-login/{provider} (disconnect)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn disconnect_social_login_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.execute_sql(&format!(
			"INSERT INTO user_social_login (user_id, provider, external_id, linked_at)
			 VALUES ('{}', 'github', 'gh-{}', NOW())",
			user.user_id,
			rand::random::<u64>(),
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DisconnectSocialLoginRequest>::builder()
				.path(DisconnectSocialLoginPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(DisconnectSocialLoginRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		!response.status_code().is_client_error()
			&& !response.status_code().is_server_error(),
		"disconnect should succeed, got {}",
		response.status_code()
	);

	let count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM user_social_login WHERE user_id = $1 AND provider = 'github'",
	)
	.bind(user.user_id)
	.fetch_one(setup.database())
	.await
	.expect("failed to count remaining links");
	assert_eq!(0, count, "disconnect should have removed the link row");
}

#[tokio::test]
async fn disconnect_social_login_not_connected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DisconnectSocialLoginRequest>::builder()
				.path(DisconnectSocialLoginPath {
					provider: SocialLoginProvider::GitHub,
				})
				.headers(DisconnectSocialLoginRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::NOT_FOUND,
		"disconnect on a non-existent link should 404"
	);
}
