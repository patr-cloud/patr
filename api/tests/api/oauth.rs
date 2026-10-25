use std::collections::BTreeMap;

use api::{models::oauth::types::OAuthAuthorizationRequest, redis::keys as redis_keys};
use axum_test::TestResponse;
use models::{ApiSuccessResponseBody, api::auth::oauth::*, utils::Uuid};
use sha2::Digest as _;

use crate::prelude::*;

/// The confidential client seeded in `setup.rs`.
const CLIENT_ID: &str = "test-client";
/// Its one registered redirect URI.
const REDIRECT_URI: &str = "http://localhost:19999/cb";
/// The S256 challenge for [`CODE_VERIFIER`].
const CODE_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

/// The `token_hash` a refresh token is stored and keyed under.
fn token_hash(refresh_token: &str) -> String {
	hex::encode(sha2::Sha256::digest(refresh_token))
}

/// Builds an `/authorize` query with every required parameter.
fn authorize_query(overrides: &[(&str, &str)]) -> String {
	let mut params = vec![
		("response_type", "code"),
		("client_id", CLIENT_ID),
		("redirect_uri", REDIRECT_URI),
		("scope", "openid profile"),
		("state", "client-state"),
		("code_challenge", CODE_CHALLENGE),
		("code_challenge_method", "S256"),
	];

	for (key, value) in overrides {
		// An empty value means "drop this parameter entirely", which is how
		// the "missing X" cases are written.
		params.retain(|(existing, _)| existing != key);
		if !value.is_empty() {
			params.push((key, value));
		}
	}

	serde_qs::to_string(&params.into_iter().collect::<BTreeMap<_, _>>())
		.expect("query should encode")
}

/// The `Location` of a redirect response.
fn location(response: &TestResponse) -> String {
	response
		.headers()
		.get(http::header::LOCATION)
		.expect("expected a redirect")
		.to_str()
		.expect("Location should be valid ASCII")
		.to_owned()
}

// ---------------------------------------------------------------------------
// GET /auth/oauth/authorize
// ---------------------------------------------------------------------------

#[tokio::test]
async fn authorize_parks_the_request_and_redirects_to_consent() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!("/auth/oauth/authorize?{}", authorize_query(&[])))
		.await;

	response.assert_status(StatusCode::SEE_OTHER);

	let location = location(&response);
	let url = reqwest::Url::parse(&location).expect("Location should be an absolute URL");
	assert_eq!(url.path(), "/authorize");

	let request_id = url
		.query_pairs()
		.find(|(key, _)| key == "requestId")
		.map(|(_, value)| value.into_owned())
		.expect("the consent URL should carry a requestId");

	assert!(
		!location.contains(CLIENT_ID) && !location.contains("code_challenge"),
		"the consent redirect leaked the client's parameters: {location}"
	);

	let stored = setup
		.get_redis_value(&redis_keys::oauth_authorization_request(
			&Uuid::parse_str(&request_id).expect("requestId should be a UUID"),
		))
		.await
		.expect("the request should be parked in redis");

	let parked: OAuthAuthorizationRequest =
		serde_json::from_str(&stored).expect("the parked request should deserialise");

	assert_eq!(parked.client_id, CLIENT_ID);
	assert_eq!(parked.redirect_uri, REDIRECT_URI);
	assert_eq!(parked.scopes, vec!["openid", "profile"]);
	assert_eq!(parked.state.as_deref(), Some("client-state"));
	assert_eq!(parked.code_challenge, CODE_CHALLENGE);
	assert!(
		!stored.contains("userId"),
		"the parked request should not name a user: {stored}"
	);
}

#[tokio::test]
async fn authorize_with_unknown_client_does_not_redirect_to_any_client_uri() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("client_id", "no-such-client")])
		))
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);
	assert!(
		response.headers().get(http::header::LOCATION).is_none(),
		"an unknown client must not produce a redirect"
	);
}

#[tokio::test]
async fn authorize_with_unregistered_redirect_uri_does_not_redirect_to_it() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("redirect_uri", "http://evil.example/steal")])
		))
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);
	assert!(
		!response.text().contains("evil.example"),
		"an unregistered redirect URI must not be echoed back"
	);
	assert!(
		response.headers().get(http::header::LOCATION).is_none(),
		"an unregistered redirect URI must not produce a redirect"
	);
}

#[tokio::test]
async fn authorize_rejects_plain_pkce() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("code_challenge_method", "plain")])
		))
		.await;

	response.assert_status(StatusCode::SEE_OTHER);
	let location = location(&response);
	assert!(location.starts_with(REDIRECT_URI), "{location}");
	assert!(location.contains("error=invalid_request"), "{location}");
	assert!(location.contains("state=client-state"), "{location}");
}

#[tokio::test]
async fn authorize_without_pkce_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("code_challenge", ""), ("code_challenge_method", "")])
		))
		.await;

	response.assert_status(StatusCode::SEE_OTHER);
	assert!(location(&response).contains("error=invalid_request"));
}

#[tokio::test]
async fn authorize_rejects_an_unsupported_response_type() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("response_type", "token")])
		))
		.await;

	response.assert_status(StatusCode::SEE_OTHER);
	assert!(location(&response).contains("error=unsupported_response_type"));
}

#[tokio::test]
async fn authorize_rejects_a_scope_the_client_may_not_request() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("scope", "openid something-else")])
		))
		.await;

	response.assert_status(StatusCode::SEE_OTHER);
	assert!(location(&response).contains("error=invalid_scope"));
}

#[tokio::test]
async fn authorize_omits_state_when_the_client_sent_none() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("state", ""), ("response_type", "token")])
		))
		.await;

	let location = location(&response);
	assert!(
		location.contains("error=unsupported_response_type"),
		"{location}"
	);
	assert!(!location.contains("state="), "{location}");
}

// ---------------------------------------------------------------------------
// GET / POST /auth/oauth/consent/{request_id}
// ---------------------------------------------------------------------------

/// Drives `/authorize` and returns the parked request's id.
async fn start_authorization(setup: &TestSetup) -> Uuid {
	let response = setup
		.make_raw_api_get(&format!("/auth/oauth/authorize?{}", authorize_query(&[])))
		.await;

	let url = reqwest::Url::parse(&location(&response)).expect("Location should parse");
	let request_id = url
		.query_pairs()
		.find(|(key, _)| key == "requestId")
		.map(|(_, value)| value.into_owned())
		.expect("missing requestId");

	Uuid::parse_str(&request_id).expect("requestId should be a UUID")
}

#[tokio::test]
async fn consent_request_renders_without_consuming_the_request() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let request_id = start_authorization(&setup).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetConsentRequestRequest>::builder()
				.path(GetConsentRequestPath { request_id })
				.headers(GetConsentRequestRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetConsentRequestResponse>>();

	assert_eq!(response.response.client_id, CLIENT_ID);
	assert_eq!(response.response.client_name, "Test Client");
	assert_eq!(response.response.redirect_uri_host, "localhost");
	assert!(!response.response.previously_approved);
	assert_eq!(response.response.scopes, vec!["openid", "profile"]);

	assert!(
		setup
			.get_redis_value(&redis_keys::oauth_authorization_request(&request_id))
			.await
			.is_some(),
		"reading the consent request must not consume it"
	);
}

#[tokio::test]
async fn consent_request_for_an_unknown_id_is_not_found() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetConsentRequestRequest>::builder()
				.path(GetConsentRequestPath {
					request_id: Uuid::now_v1(),
				})
				.headers(GetConsentRequestRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn approving_consent_issues_a_code_bound_to_the_approving_user() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let request_id = start_authorization(&setup).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SubmitConsentRequest>::builder()
				.path(SubmitConsentPath { request_id })
				.headers(SubmitConsentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(SubmitConsentRequest { approved: true })
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<SubmitConsentResponse>>();

	let redirect = reqwest::Url::parse(&response.response.redirect_uri)
		.expect("the redirect URI should parse");
	let params: std::collections::HashMap<_, _> = redirect.query_pairs().into_owned().collect();

	assert!(params.contains_key("code"), "no code was issued");
	assert_eq!(
		params.get("state").map(String::as_str),
		Some("client-state")
	);
	assert!(!params.contains_key("error"));

	assert!(
		setup
			.get_redis_value(&redis_keys::oauth_authorization_request(&request_id))
			.await
			.is_none(),
		"submitting a decision should consume the request"
	);
}

#[tokio::test]
async fn the_code_binds_the_caller_not_whoever_started_the_request() {
	let setup = setup().await.expect("failed to setup test server");
	let starter = setup.create_test_user().await;
	let approver = setup.create_test_user().await;
	let request_id = start_authorization(&setup).await;

	let _ = starter;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SubmitConsentRequest>::builder()
				.path(SubmitConsentPath { request_id })
				.headers(SubmitConsentRequestHeaders {
					authorization: approver.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(SubmitConsentRequest { approved: true })
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<SubmitConsentResponse>>();

	let redirect = reqwest::Url::parse(&response.response.redirect_uri)
		.expect("the redirect URI should parse");
	let code = redirect
		.query_pairs()
		.find(|(key, _)| key == "code")
		.map(|(_, value)| value.into_owned())
		.expect("a code should have been issued");

	let stored = setup
		.get_redis_value(&redis_keys::oauth_authorization_code(&code))
		.await
		.expect("the code payload should exist");

	assert!(
		stored.contains(&approver.user_id.to_string()),
		"the code should bind the approving user"
	);
}

#[tokio::test]
async fn denying_consent_redirects_back_with_access_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let request_id = start_authorization(&setup).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<SubmitConsentRequest>::builder()
				.path(SubmitConsentPath { request_id })
				.headers(SubmitConsentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(SubmitConsentRequest { approved: false })
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<SubmitConsentResponse>>();

	let redirect = reqwest::Url::parse(&response.response.redirect_uri)
		.expect("the redirect URI should parse");
	let params: std::collections::HashMap<_, _> = redirect.query_pairs().into_owned().collect();

	assert_eq!(
		params.get("error").map(String::as_str),
		Some("access_denied")
	);
	assert_eq!(
		params.get("state").map(String::as_str),
		Some("client-state")
	);
	assert!(!params.contains_key("code"));
}

#[tokio::test]
async fn consent_cannot_be_submitted_twice() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let request_id = start_authorization(&setup).await;

	let submit = || {
		setup.make_web_dashboard_call(
			ApiRequest::<SubmitConsentRequest>::builder()
				.path(SubmitConsentPath { request_id })
				.headers(SubmitConsentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(SubmitConsentRequest { approved: true })
				.build(),
		)
	};

	submit().await.assert_status(StatusCode::OK);
	submit().await.assert_status(StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// POST /auth/oauth/token
// ---------------------------------------------------------------------------

/// The client secret seeded in `setup.rs` for [`CLIENT_ID`].
const CLIENT_SECRET: &str = "test-client-secret";
/// The PKCE verifier from RFC 7636's worked example.
const CODE_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

/// Runs a full authorize → consent → code cycle and returns the code.
async fn issue_code(setup: &TestSetup, scope: &str) -> (String, TestUser) {
	let user = setup.create_test_user().await;

	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(&[("scope", scope)])
		))
		.await;

	let url = reqwest::Url::parse(&location(&response)).expect("Location should parse");
	let request_id = Uuid::parse_str(
		&url.query_pairs()
			.find(|(key, _)| key == "requestId")
			.map(|(_, value)| value.into_owned())
			.expect("missing requestId"),
	)
	.expect("requestId should be a UUID");

	let consent = setup
		.make_web_dashboard_call(
			ApiRequest::<SubmitConsentRequest>::builder()
				.path(SubmitConsentPath { request_id })
				.headers(SubmitConsentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(SubmitConsentRequest { approved: true })
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<SubmitConsentResponse>>();

	let redirect =
		reqwest::Url::parse(&consent.response.redirect_uri).expect("redirect should parse");
	let code = redirect
		.query_pairs()
		.find(|(key, _)| key == "code")
		.map(|(_, value)| value.into_owned())
		.expect("a code should have been issued");

	(code, user)
}

/// Exchanges a code, authenticating with HTTP Basic.
async fn exchange(setup: &TestSetup, code: &str) -> TestResponse {
	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "authorization_code"),
				("code", code),
				("redirect_uri", REDIRECT_URI),
				("code_verifier", CODE_VERIFIER),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
}

#[tokio::test]
async fn the_token_endpoint_is_rate_limited() {
	use rand::RngExt as _;
	use rustis::commands::{SortedSetCommands as _, ZAddOptions};

	let setup = setup().await.expect("failed to setup test server");
	let ip = std::net::IpAddr::V4(rand::rng().random::<u32>().into());

	// Fill this IP's per-second window, as `rate_limit.rs` does.
	let now_ns = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos() as f64;
	setup
		.state()
		.redis
		.zadd(
			redis_keys::rate_limit_ip(&ip.to_string(), 1),
			(0..50)
				.map(|i| (now_ns, format!("{now_ns}-{i}")))
				.collect::<Vec<_>>(),
			ZAddOptions::default(),
		)
		.await
		.expect("failed to fill the per-second window");

	let response = setup
		.make_raw_api_form_post_from_ip(
			"/auth/oauth/token",
			&[
				("grant_type", "authorization_code"),
				("code", "no-such-code"),
				("redirect_uri", REDIRECT_URI),
				("code_verifier", CODE_VERIFIER),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
			ip,
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::TOO_MANY_REQUESTS);
	assert!(response.text().contains("temporarily_unavailable"));
}

#[tokio::test]
async fn exchanging_a_code_returns_tokens() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid profile offline_access").await;

	let response = exchange(&setup, &code).await;
	response.assert_status(StatusCode::OK);

	let body: serde_json::Value = response.json();
	assert!(
		body.get("success").is_none(),
		"the token response must not be wrapped in Patr's envelope: {body}"
	);
	assert!(body["access_token"].is_string());
	assert_eq!(body["token_type"], "Bearer");
	assert!(body["expires_in"].is_number());
	assert!(body["refresh_token"].is_string());
	assert_eq!(body["scope"], "openid profile offline_access");

	assert_eq!(
		response
			.headers()
			.get(http::header::CACHE_CONTROL)
			.and_then(|value| value.to_str().ok()),
		Some("no-store")
	);
}

#[tokio::test]
async fn the_access_token_is_an_es256_jwt_naming_the_api_as_its_audience() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, user) = issue_code(&setup, "openid offline_access").await;

	let body: serde_json::Value = exchange(&setup, &code).await.json();
	let token = body["access_token"].as_str().expect("an access token");

	let header = jsonwebtoken::decode_header(token).expect("the header should parse");
	assert_eq!(header.alg, jsonwebtoken::Algorithm::ES256);
	assert!(header.kid.is_some(), "the header must name the signing key");
	assert_eq!(header.typ.as_deref(), Some("at+jwt"));

	let claims = token.split('.').nth(1).expect("a payload segment");
	let decoded = base64::Engine::decode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, claims)
		.expect("the payload should be base64url");
	let claims: serde_json::Value =
		serde_json::from_slice(&decoded).expect("the payload should be JSON");

	assert_eq!(claims["sub"], user.user_id.to_string());
	assert_ne!(claims["sid"], claims["sub"]);
	assert_eq!(claims["azp"], CLIENT_ID);
}

#[tokio::test]
async fn a_code_cannot_be_exchanged_twice() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid offline_access").await;

	exchange(&setup, &code).await.assert_status(StatusCode::OK);

	let replayed = exchange(&setup, &code).await;
	replayed.assert_status(StatusCode::BAD_REQUEST);
	assert!(replayed.text().contains("invalid_grant"));
}

#[tokio::test]
async fn the_wrong_code_verifier_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid").await;

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "authorization_code"),
				("code", &code),
				("redirect_uri", REDIRECT_URI),
				("code_verifier", "not-the-verifier-that-was-used"),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);
	assert!(response.text().contains("invalid_grant"));
}

#[tokio::test]
async fn a_mismatched_redirect_uri_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid").await;

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "authorization_code"),
				("code", &code),
				("redirect_uri", "http://localhost:19999/somewhere-else"),
				("code_verifier", CODE_VERIFIER),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);
	assert!(response.text().contains("invalid_grant"));
}

#[tokio::test]
async fn client_credentials_are_accepted_in_the_body_as_well_as_basic() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid").await;

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "authorization_code"),
				("client_id", CLIENT_ID),
				("client_secret", CLIENT_SECRET),
				("code", &code),
				("redirect_uri", REDIRECT_URI),
				("code_verifier", CODE_VERIFIER),
			],
			None,
		)
		.await;

	response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn a_wrong_client_secret_is_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid").await;

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "authorization_code"),
				("code", &code),
				("redirect_uri", REDIRECT_URI),
				("code_verifier", CODE_VERIFIER),
			],
			Some((CLIENT_ID, "not-the-secret")),
		)
		.await;

	response.assert_status(StatusCode::UNAUTHORIZED);
	assert!(
		response
			.headers()
			.get(http::header::WWW_AUTHENTICATE)
			.is_some()
	);
	assert!(response.text().contains("invalid_client"));
}

#[tokio::test]
async fn a_code_cannot_be_redeemed_by_another_client() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid").await;

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "authorization_code"),
				("client_id", "test-public-client"),
				("code", &code),
				("redirect_uri", REDIRECT_URI),
				("code_verifier", CODE_VERIFIER),
			],
			None,
		)
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);
	assert!(response.text().contains("invalid_grant"));
}

#[tokio::test]
async fn no_refresh_token_without_offline_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid profile").await;

	let body: serde_json::Value = exchange(&setup, &code).await.json();
	assert!(
		body.get("refresh_token").is_none(),
		"a refresh token should only be issued for offline_access: {body}"
	);
}

#[tokio::test]
async fn refreshing_rotates_the_token_and_retires_the_old_one() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid offline_access").await;

	let first: serde_json::Value = exchange(&setup, &code).await.json();
	let original = first["refresh_token"].as_str().expect("a refresh token");

	let second = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[("grant_type", "refresh_token"), ("refresh_token", original)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await;
	second.assert_status(StatusCode::OK);
	let second: serde_json::Value = second.json();
	let rotated = second["refresh_token"].as_str().expect("a rotated token");
	assert_ne!(rotated, original, "the refresh token should rotate");
}

#[tokio::test]
async fn a_raced_refresh_token_replays_the_pair_it_already_minted() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid offline_access").await;

	let first: serde_json::Value = exchange(&setup, &code).await.json();
	let original = first["refresh_token"]
		.as_str()
		.expect("a refresh token")
		.to_owned();

	let second: serde_json::Value = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &original),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.json();
	let rotated = second["refresh_token"]
		.as_str()
		.expect("a rotated token")
		.to_owned();

	let raced: serde_json::Value = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &original),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.json();
	assert_eq!(
		raced["refresh_token"].as_str(),
		Some(rotated.as_str()),
		"a raced refresh should return the refresh token the loser already minted"
	);
	assert_eq!(
		raced["access_token"], second["access_token"],
		"and the access token that came with it, so both racers hold one pair"
	);

	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[("grant_type", "refresh_token"), ("refresh_token", &rotated)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn replaying_a_refresh_token_outside_the_grace_window_revokes_the_grant() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid offline_access").await;

	let first: serde_json::Value = exchange(&setup, &code).await.json();
	let original = first["refresh_token"]
		.as_str()
		.expect("a refresh token")
		.to_owned();
	let original_hash = token_hash(&original);

	let second: serde_json::Value = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &original),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.json();
	let rotated = second["refresh_token"]
		.as_str()
		.expect("a rotated token")
		.to_owned();

	setup
		.execute_sql(&format!(
			"UPDATE oauth_refresh_token SET consumed = NOW() - INTERVAL '1 hour' \
			 WHERE token_hash = '{original_hash}'"
		))
		.await;

	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &original),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);

	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[("grant_type", "refresh_token"), ("refresh_token", &rotated)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_lost_replacement_in_redis_refuses_the_refresh_without_revoking() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid offline_access").await;

	let first: serde_json::Value = exchange(&setup, &code).await.json();
	let original = first["refresh_token"]
		.as_str()
		.expect("a refresh token")
		.to_owned();

	let second: serde_json::Value = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &original),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.json();
	let rotated = second["refresh_token"]
		.as_str()
		.expect("a rotated token")
		.to_owned();

	setup
		.delete_redis_value(&redis_keys::oauth_replacement_tokens(&token_hash(
			&original,
		)))
		.await;

	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &original),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);

	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[("grant_type", "refresh_token"), ("refresh_token", &rotated)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn an_unsupported_grant_type_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[("grant_type", "client_credentials")],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);
	assert!(response.text().contains("unsupported_grant_type"));
}
