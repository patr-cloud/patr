use std::collections::BTreeMap;

use api::{models::oauth::types::OAuthAuthorizationRequest, redis::keys as redis_keys};
use axum_test::TestResponse;
use models::{
	ApiSuccessResponseBody,
	api::{auth::oauth::*, user::*},
	utils::Uuid,
};
use time::{Duration, OffsetDateTime};

use crate::prelude::*;

/// The confidential client seeded in `setup.rs`.
const CLIENT_ID: &str = "test-client";
/// Its one registered redirect URI.
const REDIRECT_URI: &str = "http://localhost:19999/cb";
/// The S256 challenge for [`CODE_VERIFIER`], i.e.
/// `base64url(sha256(verifier))` with the padding stripped.
const CODE_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

/// Pulls the `sid` — the grant id — out of an access token.
///
/// Read without verifying: the test only needs to name the row it is about
/// to reach into, and the server is what decides whether the token is good.
fn grant_id_of(access_token: &str) -> Uuid {
	let payload = access_token.split('.').nth(1).expect("a JWT payload");
	let decoded = base64::Engine::decode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, payload)
		.expect("a base64url payload");
	let claims: serde_json::Value = serde_json::from_slice(&decoded).expect("a JSON payload");

	Uuid::parse_str(claims["sid"].as_str().expect("a `sid` claim")).expect("`sid` is a UUID")
}

/// Pulls the token id out of a `patrv1.{secret}.{id}` refresh token.
fn token_id(refresh_token: &str) -> Uuid {
	refresh_token
		.rsplit('.')
		.next()
		.and_then(|id| Uuid::parse_str(id).ok())
		.expect("a refresh token ending in its id")
}

/// Builds an `/authorize` query with every required parameter, so each test
/// can override exactly the one it is about and nothing else.
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

	// The client's own parameters must not reach the dashboard's address bar
	// — only the opaque id does.
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
	// Deliberately absent: the user is bound at consent time from their own
	// session, so a leaked request id cannot bind someone else's account.
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

	// RFC 6749 4.1.2.1: with no verified client there is no URI worth
	// trusting, so this must not become an open redirect.
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
	// The bug this guards against is redirect-URI injection: bouncing the
	// browser to an unregistered URI hands the code to whoever asked.
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

	// OAuth 2.1 removes `plain`: the verifier would equal the challenge sent
	// in the clear, leaving PKCE protecting nothing.
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

	// PKCE is mandatory for every client under OAuth 2.1, not just public
	// ones.
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

	// The implicit flow is gone in OAuth 2.1.
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

	// `state=` is not the same as no state — a client comparing it against
	// what it stored would reject a value it never sent.
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

	// A reload, or SSR followed by hydration, both hit this twice — so it
	// must not consume the request.
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

	// Consumed, so a decision cannot be replayed into a second code.
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

	// The parked request carries no user at all, so whoever actually
	// approves it is the one bound. This is what stops a leaked request id
	// being used to attach a grant to somebody else's account.
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

	// A refusal still goes back to the client; leaving it to time out would
	// hang the user on a blank page instead of the client's own handling.
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
/// The PKCE verifier whose challenge is [`CODE_CHALLENGE`]. The verifier is
/// RFC 7636's own worked example.
const CODE_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

/// Runs a full authorize → consent → code cycle and returns the code.
async fn issue_code(setup: &TestSetup, scope: &str) -> (String, TestUser) {
	issue_code_with(setup, &[("scope", scope)]).await
}

/// The same, but with the whole `/authorize` query open to overrides, for the
/// tests that care about a parameter other than `scope` — `nonce`, say.
async fn issue_code_with(setup: &TestSetup, overrides: &[(&str, &str)]) -> (String, TestUser) {
	let user = setup.create_test_user().await;
	let code = issue_code_for_with(setup, &user, overrides).await;

	(code, user)
}

/// Runs the cycle again for a user who already exists, so a test can give one
/// user two grants for the same app.
async fn issue_code_for(setup: &TestSetup, user: &TestUser, scope: &str) -> String {
	issue_code_for_with(setup, user, &[("scope", scope)]).await
}

/// The shared body of the three helpers above.
async fn issue_code_for_with(
	setup: &TestSetup,
	user: &TestUser,
	overrides: &[(&str, &str)],
) -> String {
	let response = setup
		.make_raw_api_get(&format!(
			"/auth/oauth/authorize?{}",
			authorize_query(overrides)
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

	code
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
	use futures::StreamExt as _;
	use rand::RngExt as _;

	let setup = setup().await.expect("failed to setup test server");

	// The back-channel routes get none of the layer stack a
	// `declare_api_endpoint!` one does, so without an explicit check they
	// would be unthrottled — and this one takes secrets without a session.
	// Pinned to one IP, since the point is to exhaust a bucket.
	let ip = std::net::IpAddr::V4(rand::rng().random::<u32>().into());
	let form = [
		("grant_type", "authorization_code"),
		("code", "no-such-code"),
		("redirect_uri", REDIRECT_URI),
		("code_verifier", CODE_VERIFIER),
	];
	let attempt = || {
		setup.make_raw_api_form_post_from_ip(
			"/auth/oauth/token",
			&form,
			Some((CLIENT_ID, CLIENT_SECRET)),
			ip,
		)
	};

	// The debug-build per-second window, matching `rate_limit.rs`.
	futures::stream::iter(0..50)
		.map(|_| attempt())
		.buffer_unordered(25)
		.collect::<Vec<_>>()
		.await;

	let response = attempt().await;

	assert_eq!(response.status_code(), StatusCode::TOO_MANY_REQUESTS);
	assert!(response.text().contains("temporarily_unavailable"));
}

#[tokio::test]
async fn exchanging_a_code_returns_tokens() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid profile offline_access").await;

	let response = exchange(&setup, &code).await;
	response.assert_status(StatusCode::OK);

	// The response must be a bare body, not Patr's success envelope — a
	// client library reads these names off the top level.
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

	// Tokens must never be cached by anything in the path.
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
	// RFC 9068. Together with `aud`, this is what stops an id token — which
	// names the client as its audience — being replayed as an API credential.
	assert_eq!(header.typ.as_deref(), Some("at+jwt"));

	// Decoded without verification: the point here is which claims were put
	// in, not whether the signature holds (covered by the JWKS unit test).
	let claims = token.split('.').nth(1).expect("a payload segment");
	let decoded = base64::Engine::decode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, claims)
		.expect("the payload should be base64url");
	let claims: serde_json::Value =
		serde_json::from_slice(&decoded).expect("the payload should be JSON");

	// `sub` is the user and `sid` is the grant. Getting these the wrong way
	// round would make relying parties treat each new grant as a new account.
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

	// The whole point of PKCE: holding the code is not enough.
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

	// Grafana's oauth2 library tries Basic first and falls back to body
	// fields; supporting only one would fail against it.
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

	// RFC 6749 5.2: a client that fails to authenticate gets 401 and a
	// challenge, so it can tell bad credentials from a bad request. Unlike
	// /authorize, the caller here is a server, not a browser.
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

	// Authenticates fine as itself, but the code belongs to someone else.
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

	// Within the grace window a replay is treated as a race and handed the
	// same pair back — that is what stops ordinary client concurrency from
	// logging people out. The grant must still be alive afterwards.
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

	// The rotated token still works, so nothing was revoked by the race.
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
	let original_id = token_id(&original);

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

	// Backdate the consumption so the replay lands outside the window. Past
	// it there is no benign reading left: the token was used once and is
	// being presented again, which is what a stolen token looks like.
	setup
		.execute_sql(&format!(
			"UPDATE oauth_refresh_token SET consumed = NOW() - INTERVAL '1 hour' \
			 WHERE id = '{original_id}'"
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

	// The whole family dies with the replay, so the token the attacker did
	// not have is dead too.
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

	// Simulate Redis losing the pair — a flush, an eviction, a failover.
	setup
		.delete_redis_value(&redis_keys::oauth_replacement_tokens(&token_id(&original)))
		.await;

	// Still inside the window, so this is not evidence of anything. The one
	// request fails and the client retries; the grant survives.
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
async fn a_forged_secret_against_a_real_token_id_does_not_revoke_the_grant() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid offline_access").await;

	let first: serde_json::Value = exchange(&setup, &code).await.json();
	let original = first["refresh_token"]
		.as_str()
		.expect("a refresh token")
		.to_owned();

	// Same token id, wrong secret. This is a guess, not a replay — treating
	// it as one would let anyone kill a victim's grant by iterating ids.
	let token_id = original.rsplit('.').next().expect("a token id");
	let forged = format!("patrv1.{}.{}", Uuid::new_v4(), token_id);

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[("grant_type", "refresh_token"), ("refresh_token", &forged)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await;
	response.assert_status(StatusCode::BAD_REQUEST);

	// The real token must still work.
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
		.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn an_unsupported_grant_type_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");

	// client_credentials has no user, so there is no permission map to act
	// with under this scope model. It is deliberately not implemented.
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

/// Runs the whole flow and hands back a usable access token.
async fn issue_access_token(setup: &TestSetup, scope: &str) -> (String, TestUser) {
	let (code, user) = issue_code(setup, scope).await;
	let tokens: serde_json::Value = exchange(setup, &code).await.json();

	(
		tokens["access_token"]
			.as_str()
			.expect("an access token")
			.to_owned(),
		user,
	)
}

#[tokio::test]
async fn an_oauth_access_token_authenticates_the_api() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, user) = issue_access_token(&setup, "openid profile email").await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetUserInfoRequest)
				.build(),
		)
		.await;

	response.assert_status(StatusCode::OK);
	let body = response.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();
	assert_eq!(body.response.basic_user_info.id, user.user_id);
	assert_eq!(body.response.email, user.email);
}

#[tokio::test]
async fn an_oauth_grant_sees_the_same_workspaces_as_the_user() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, user) = issue_access_token(&setup, "openid").await;

	// No ceiling: the grant acts with the user's full current authority, so
	// the two views have to agree exactly. If a ceiling is ever added this is
	// the test that will notice.
	let via_grant = setup
		.make_api_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.path(ListUserWorkspacesPath)
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ListUserWorkspacesRequest)
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	let via_session = setup
		.make_web_dashboard_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.path(ListUserWorkspacesPath)
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ListUserWorkspacesRequest)
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUserWorkspacesResponse>>();

	assert_eq!(
		via_grant
			.response
			.workspaces
			.iter()
			.map(|workspace| workspace.id)
			.collect::<Vec<_>>(),
		via_session
			.response
			.workspaces
			.iter()
			.map(|workspace| workspace.id)
			.collect::<Vec<_>>(),
	);
}

#[tokio::test]
async fn an_api_false_endpoint_rejects_an_oauth_token() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _user) = issue_access_token(&setup, "openid").await;

	// Listing (and so revoking) the user's browser sessions is exactly the
	// kind of thing a grant must not reach. On the cloud build these
	// endpoints are filtered out of the API arm at mount time; self-hosted
	// mounts the whole API as WebDashboard, so the authenticator is the only
	// thing standing here — which is why this goes through the web arm.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListWebLoginsRequest>::builder()
				.path(ListWebLoginsPath)
				.headers(ListWebLoginsRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ListWebLoginsRequest)
				.build(),
		)
		.await;

	response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn an_id_token_presented_as_a_bearer_token_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, user) = issue_access_token(&setup, "openid").await;

	// Forge the thing PR 6 will legitimately mint: same key, same `kid`,
	// same signature — only `typ` and `aud` differ. Relying parties treat id
	// tokens as non-secret and write them to logs and cookies, so this is
	// the realistic attack, not a malformed-token one.
	let key = api::models::oauth::keys::get_signing_key(&setup.state().config)
		.expect("a signing key should exist");

	let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
	header.kid = Some(key.kid.clone());
	header.typ = Some("JWT".to_owned());

	let now = OffsetDateTime::now_utc();
	let id_token = jsonwebtoken::encode(
		&header,
		&serde_json::json!({
			"iss": format!("https://api.{}", setup.state().config.server.base_domain),
			"sub": user.user_id.to_string(),
			// An id token names the client, not the API.
			"aud": CLIENT_ID,
			"exp": (now + Duration::minutes(5)).unix_timestamp(),
			"iat": now.unix_timestamp(),
		}),
		&key.encoding_key,
	)
	.expect("failed to mint an id token");

	setup
		.make_api_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&id_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetUserInfoRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);

	// And the real access token still works, so the rejection was about the
	// token rather than the setup.
	setup
		.make_api_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetUserInfoRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn revoking_a_grant_stops_its_access_token_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _user) = issue_access_token(&setup, "openid").await;

	let grant_id = grant_id_of(&access_token);

	setup
		.execute_sql(&format!(
			"UPDATE oauth_login SET revoked = NOW() WHERE login_id = '{grant_id}'"
		))
		.await;

	// The JWT is still perfectly valid and unexpired. What kills it is the
	// grant row being read on every request.
	setup
		.make_api_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetUserInfoRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn userinfo_answers_get_and_post() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, user) = issue_access_token(&setup, "openid").await;

	for response in [
		setup
			.make_raw_api_get_authed("/auth/oauth/userinfo", &access_token)
			.await,
		setup
			.make_raw_api_post_authed("/auth/oauth/userinfo", &access_token)
			.await,
	] {
		response.assert_status(StatusCode::OK);
		let claims: serde_json::Value = response.json();
		assert_eq!(
			claims["sub"].as_str(),
			Some(user.user_id.to_string().as_str())
		);
	}
}

#[tokio::test]
async fn userinfo_returns_only_the_claims_the_scopes_allow() {
	let setup = setup().await.expect("failed to setup test server");

	let (openid_only, _user) = issue_access_token(&setup, "openid").await;
	let claims: serde_json::Value = setup
		.make_raw_api_get_authed("/auth/oauth/userinfo", &openid_only)
		.await
		.json();
	assert!(claims["sub"].is_string(), "`sub` is never gated");
	assert!(
		claims.get("email").is_none(),
		"`email` needs the email scope"
	);
	assert!(
		claims.get("name").is_none(),
		"`name` needs the profile scope"
	);

	let (full, user) = issue_access_token(&setup, "openid profile email").await;
	let claims: serde_json::Value = setup
		.make_raw_api_get_authed("/auth/oauth/userinfo", &full)
		.await
		.json();
	assert_eq!(claims["email"].as_str(), Some(user.email.as_str()));
	assert_eq!(claims["email_verified"].as_bool(), Some(true));
	assert!(claims["name"].is_string());
	assert!(claims["given_name"].is_string());
	assert!(claims["family_name"].is_string());
}

#[tokio::test]
async fn userinfo_without_a_token_challenges_for_a_bearer() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup.make_raw_api_get("/auth/oauth/userinfo").await;

	response.assert_status(StatusCode::UNAUTHORIZED);
	// RFC 6750 section 3: without the challenge a client cannot tell an
	// expired token from a request it should not retry.
	let challenge = response
		.headers()
		.get("www-authenticate")
		.expect("a WWW-Authenticate challenge")
		.to_str()
		.expect("a printable challenge");
	assert!(challenge.starts_with("Bearer "), "got `{challenge}`");
}

#[tokio::test]
async fn a_web_dashboard_jwt_is_still_rejected_by_the_api_arm() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	// The OAuth branch classifies on ES256 and must not have widened the
	// api arm to the dashboard's HS256 sessions on its way past.
	setup
		.make_api_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetUserInfoRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);
}

/// Fetches the published JWKS and finds the key a token's header names.
///
/// Deliberately the long way round — header `kid`, then look it up in the
/// document a relying party would have fetched — because that is the path a
/// real client takes, and it is what catches a JWK that is published but not
/// selectable.
async fn decoding_key_for(setup: &TestSetup, token: &str) -> jsonwebtoken::DecodingKey {
	let jwks: jsonwebtoken::jwk::JwkSet = setup
		.make_raw_api_get("/.well-known/jwks.json")
		.await
		.json();

	let kid = jsonwebtoken::decode_header(token)
		.expect("a decodable header")
		.kid
		.expect("a `kid` naming the signing key");

	let jwk = jwks.find(&kid).expect("the JWKS should publish that `kid`");

	jsonwebtoken::DecodingKey::from_jwk(jwk).expect("the JWK should build a decoding key")
}

#[tokio::test]
async fn the_discovery_document_advertises_only_what_is_mounted() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_raw_api_get("/.well-known/openid-configuration")
		.await;
	response.assert_status(StatusCode::OK);
	let doc: serde_json::Value = response.json();

	let issuer = doc["issuer"].as_str().expect("an issuer").to_owned();

	// Every endpoint the document names has to actually answer, and every
	// endpoint that answers has to be named. Listing one that does not exist
	// is worse than omitting it — a client library will happily POST to it.
	let mut endpoints = doc
		.as_object()
		.expect("an object")
		.keys()
		.filter(|key| key.ends_with("_endpoint"))
		.cloned()
		.collect::<Vec<_>>();
	endpoints.sort();
	assert_eq!(
		endpoints,
		[
			"authorization_endpoint",
			"introspection_endpoint",
			"revocation_endpoint",
			"token_endpoint",
			"userinfo_endpoint"
		],
		"a new endpoint has to be advertised here, and an unmounted one must not be"
	);

	for (key, path) in [
		("authorization_endpoint", "/auth/oauth/authorize"),
		("token_endpoint", "/auth/oauth/token"),
		("userinfo_endpoint", "/auth/oauth/userinfo"),
		("revocation_endpoint", "/auth/oauth/revoke"),
		("introspection_endpoint", "/auth/oauth/introspect"),
		("jwks_uri", "/.well-known/jwks.json"),
	] {
		assert_eq!(
			doc[key].as_str(),
			Some(format!("{issuer}{path}").as_str()),
			"{key} should hang off the issuer"
		);
	}

	// OAuth 2.1 removes `plain`, and advertising it would invite a client to
	// downgrade to it.
	assert_eq!(
		doc["code_challenge_methods_supported"],
		serde_json::json!(["S256"])
	);
	assert_eq!(doc["response_types_supported"], serde_json::json!(["code"]));
	assert_eq!(
		doc["id_token_signing_alg_values_supported"],
		serde_json::json!(["ES256"])
	);
	assert_eq!(
		doc["subject_types_supported"],
		serde_json::json!(["public"])
	);
}

#[tokio::test]
async fn the_jwks_publishes_selectable_keys_and_no_private_material() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup.make_raw_api_get("/.well-known/jwks.json").await;
	response.assert_status(StatusCode::OK);
	let jwks: serde_json::Value = response.json();

	let keys = jwks["keys"].as_array().expect("a keys array");
	assert!(!keys.is_empty(), "at least one key should be published");

	for key in keys {
		// The one thing that must never appear here. `d` is the private
		// scalar; publishing it would hand out the ability to mint tokens.
		assert!(
			key.get("d").is_none(),
			"a published JWK must carry no private material: {key}"
		);

		// A client picks a key by `kid`, then filters on `use` and `alg`. A
		// JWK missing any of them is silently skipped and verification fails
		// with an unhelpful "no applicable key".
		assert!(key["kid"].is_string(), "missing `kid`: {key}");
		assert_eq!(key["use"].as_str(), Some("sig"), "missing `use`: {key}");
		assert_eq!(key["alg"].as_str(), Some("ES256"), "missing `alg`: {key}");
		assert_eq!(key["kty"].as_str(), Some("EC"), "wrong `kty`: {key}");
		assert_eq!(key["crv"].as_str(), Some("P-256"), "wrong `crv`: {key}");
	}
}

#[tokio::test]
async fn the_id_token_verifies_against_the_published_jwks() {
	let setup = setup().await.expect("failed to setup test server");
	const NONCE: &str = "n-0S6_WzA2Mj";

	let (code, user) = issue_code_with(
		&setup,
		&[("scope", "openid profile email"), ("nonce", NONCE)],
	)
	.await;

	let tokens: serde_json::Value = exchange(&setup, &code).await.json();
	let id_token = tokens["id_token"].as_str().expect("an id token").to_owned();
	let access_token = tokens["access_token"].as_str().expect("an access token");

	// Exactly what a relying party does: fetch the JWKS, pick the key the
	// header names, and insist on the algorithm, issuer and audience.
	let key = decoding_key_for(&setup, &id_token).await;
	let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
	validation.set_audience(&[CLIENT_ID]);
	validation.set_issuer(&[format!(
		"https://api.{}",
		setup.state().config.server.base_domain
	)]);

	let claims = jsonwebtoken::decode::<serde_json::Value>(&id_token, &key, &validation)
		.expect("the id token should verify against the published JWKS")
		.claims;

	assert_eq!(
		claims["sub"].as_str(),
		Some(user.user_id.to_string().as_str())
	);
	assert_eq!(
		claims["nonce"].as_str(),
		Some(NONCE),
		"the nonce ties this token to the request that started the flow"
	);
	assert_eq!(claims["email"].as_str(), Some(user.email.as_str()));
	assert!(claims["name"].is_string());
	assert!(claims["auth_time"].is_number());

	// OIDC Core section 3.1.3.6: left-most 128 bits of SHA-256, base64url.
	// Computed independently here, so a wrong slice or encoding shows up.
	let expected = base64::Engine::encode(
		&base64::prelude::BASE64_URL_SAFE_NO_PAD,
		&<sha2::Sha256 as sha2::Digest>::digest(access_token.as_bytes())[..16],
	);
	assert_eq!(
		claims["at_hash"].as_str(),
		Some(expected.as_str()),
		"at_hash should bind the id token to the access token it came with"
	);
}

#[tokio::test]
async fn a_tampered_id_token_fails_verification() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid").await;

	let tokens: serde_json::Value = exchange(&setup, &code).await.json();
	let id_token = tokens["id_token"].as_str().expect("an id token").to_owned();
	let key = decoding_key_for(&setup, &id_token).await;

	// Flip one character of the signature. Everything else about the token is
	// untouched, so only the signature check can catch this.
	let (rest, signature) = id_token.rsplit_once('.').expect("a JWT signature");
	let flipped = signature
		.chars()
		.next()
		.map(|first| if first == 'A' { 'B' } else { 'A' })
		.expect("a non-empty signature");
	let tampered = format!("{rest}.{flipped}{}", &signature[1..]);

	jsonwebtoken::decode::<serde_json::Value>(
		&tampered,
		&key,
		&jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256),
	)
	.expect_err("a tampered id token must not verify");
}

#[tokio::test]
async fn a_grant_without_openid_is_plain_oauth() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "offline_access").await;

	// `openid` is what makes an authorization a login. Without it the client
	// still gets its tokens, just no id token — this is an OAuth 2.1 server
	// with OIDC layered on, not an OIDC-only one.
	let body: serde_json::Value = exchange(&setup, &code).await.json();

	assert!(body["access_token"].is_string());
	assert!(body["refresh_token"].is_string());
	assert!(body.get("id_token").is_none());
}

#[tokio::test]
async fn a_refresh_does_not_return_an_id_token() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid offline_access").await;

	let first: serde_json::Value = exchange(&setup, &code).await.json();
	assert!(first["id_token"].is_string(), "the exchange mints one");
	let refresh_token = first["refresh_token"].as_str().expect("a refresh token");

	// OIDC Core section 12.2 allows the refresh response to omit it, and
	// echoing the original nonce onto a later token would defeat the point of
	// having one.
	let refreshed: serde_json::Value = setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", refresh_token),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.json();

	assert!(refreshed["access_token"].is_string());
	assert!(refreshed.get("id_token").is_none());
}

#[tokio::test]
async fn a_real_id_token_is_rejected_as_an_api_credential() {
	let setup = setup().await.expect("failed to setup test server");
	let (code, _user) = issue_code(&setup, "openid").await;

	let tokens: serde_json::Value = exchange(&setup, &code).await.json();
	let id_token = tokens["id_token"].as_str().expect("an id token");

	// The genuine article this time, not a hand-rolled forgery: correctly
	// signed by a live key, but `typ: JWT` and the client as its audience.
	// Relying parties treat these as non-secret and put them in logs and
	// cookies, so this is the realistic path to an API credential.
	setup
		.make_api_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(id_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetUserInfoRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);
}

/// The public client seeded in `setup.rs`, for the tests that need a caller
/// that is *not* the one a grant belongs to.
const PUBLIC_CLIENT_ID: &str = "test-public-client";

/// Issues a grant and hands back both halves of the pair plus the user.
async fn issue_token_pair(setup: &TestSetup) -> (String, String, TestUser) {
	let (code, user) = issue_code(setup, "openid offline_access").await;
	let tokens: serde_json::Value = exchange(setup, &code).await.json();

	(
		tokens["access_token"]
			.as_str()
			.expect("an access token")
			.to_owned(),
		tokens["refresh_token"]
			.as_str()
			.expect("a refresh token")
			.to_owned(),
		user,
	)
}

/// Calls `GetUserInfo` on the api arm with a bearer token.
async fn call_api_with(setup: &TestSetup, access_token: &str) -> TestResponse {
	setup
		.make_api_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetUserInfoRequest)
				.build(),
		)
		.await
}

#[tokio::test]
async fn revoking_a_refresh_token_kills_the_whole_grant() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, refresh_token, _user) = issue_token_pair(&setup).await;

	setup
		.make_raw_api_form_post(
			"/auth/oauth/revoke",
			&[("token", &refresh_token)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::OK);

	// Both halves die together. RFC 7009 section 2.1 permits this, and it is
	// what "log this app out" has to mean.
	call_api_with(&setup, &access_token)
		.await
		.assert_status(StatusCode::UNAUTHORIZED);

	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &refresh_token),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn revoking_an_access_token_kills_the_whole_grant() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, refresh_token, _user) = issue_token_pair(&setup).await;

	setup
		.make_raw_api_form_post(
			"/auth/oauth/revoke",
			&[
				("token", &access_token),
				("token_type_hint", "access_token"),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::OK);

	setup
		.make_raw_api_form_post(
			"/auth/oauth/token",
			&[
				("grant_type", "refresh_token"),
				("refresh_token", &refresh_token),
			],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn revoking_an_unknown_token_succeeds_silently() {
	let setup = setup().await.expect("failed to setup test server");

	// RFC 7009 section 2.2. A 400 here would turn the endpoint into an oracle
	// for whether a token someone found is real.
	for token in [
		"patrv1.deadbeef.notauuid",
		"not-a-token-at-all",
		&format!("patrv1.{}.{}", Uuid::new_v4(), Uuid::new_v4()),
	] {
		setup
			.make_raw_api_form_post(
				"/auth/oauth/revoke",
				&[("token", token)],
				Some((CLIENT_ID, CLIENT_SECRET)),
			)
			.await
			.assert_status(StatusCode::OK);
	}
}

#[tokio::test]
async fn another_clients_token_cannot_be_revoked() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, refresh_token, _user) = issue_token_pair(&setup).await;

	// Reported as a success, exactly like an unknown token, so the caller
	// learns nothing about whether it exists.
	setup
		.make_raw_api_form_post(
			"/auth/oauth/revoke",
			&[("token", &refresh_token), ("client_id", PUBLIC_CLIENT_ID)],
			None,
		)
		.await
		.assert_status(StatusCode::OK);

	// And the grant is untouched.
	call_api_with(&setup, &access_token)
		.await
		.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn revoke_rejects_a_hint_for_a_token_type_we_do_not_issue() {
	let setup = setup().await.expect("failed to setup test server");
	let (_access_token, refresh_token, _user) = issue_token_pair(&setup).await;

	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/revoke",
			&[("token", &refresh_token), ("token_type_hint", "pac_token")],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);
	assert!(response.text().contains("unsupported_token_type"));
}

#[tokio::test]
async fn introspection_describes_a_live_access_token() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _refresh_token, user) = issue_token_pair(&setup).await;

	let body: serde_json::Value = setup
		.make_raw_api_form_post(
			"/auth/oauth/introspect",
			&[("token", &access_token)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.json();

	assert_eq!(body["active"].as_bool(), Some(true));
	assert_eq!(body["client_id"].as_str(), Some(CLIENT_ID));
	assert_eq!(
		body["sub"].as_str(),
		Some(user.user_id.to_string().as_str())
	);
	assert_eq!(body["token_type"].as_str(), Some("Bearer"));
	assert!(body["exp"].is_number());
	assert!(body["scope"].as_str().is_some_and(|s| s.contains("openid")));
}

#[tokio::test]
async fn introspection_reports_a_revoked_token_as_inactive() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _refresh_token, _user) = issue_token_pair(&setup).await;

	setup
		.make_raw_api_form_post(
			"/auth/oauth/revoke",
			&[("token", &access_token)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.assert_status(StatusCode::OK);

	let body: serde_json::Value = setup
		.make_raw_api_form_post(
			"/auth/oauth/introspect",
			&[("token", &access_token)],
			Some((CLIENT_ID, CLIENT_SECRET)),
		)
		.await
		.json();

	assert_eq!(body["active"].as_bool(), Some(false));
	// RFC 7662 section 2.2: an inactive response carries nothing else, so it
	// cannot be used to tell "revoked" from "never existed".
	assert!(body.get("sub").is_none());
	assert!(body.get("client_id").is_none());
	assert!(body.get("exp").is_none());
}

#[tokio::test]
async fn introspection_says_nothing_about_another_clients_token() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _refresh_token, _user) = issue_token_pair(&setup).await;

	// `test-public-client` has no secret, so it is refused outright — but even
	// a second confidential client would only get `active: false` here.
	let response = setup
		.make_raw_api_form_post(
			"/auth/oauth/introspect",
			&[("token", &access_token), ("client_id", PUBLIC_CLIENT_ID)],
			None,
		)
		.await;

	response.assert_status(StatusCode::UNAUTHORIZED);
	assert!(response.text().contains("invalid_client"));
}

#[tokio::test]
async fn listing_grants_shows_the_app_behind_each_one() {
	let setup = setup().await.expect("failed to setup test server");
	let (_access_token, _refresh_token, user) = issue_token_pair(&setup).await;

	let body = setup
		.make_web_dashboard_call(
			ApiRequest::<ListOAuthGrantsRequest>::builder()
				.path(ListOAuthGrantsPath)
				.headers(ListOAuthGrantsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ListOAuthGrantsRequest)
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListOAuthGrantsResponse>>();

	let grant = body
		.response
		.grants
		.first()
		.expect("the grant just created should be listed");

	assert_eq!(grant.client_id, CLIENT_ID);
	// Joined from `oauth_client`, which is why that table exists: the screen
	// has to render an app name rather than a bare id.
	assert!(!grant.client_name.is_empty());
	assert!(grant.scope.contains("openid"));
}

#[tokio::test]
async fn revoking_a_grant_from_the_dashboard_cuts_the_app_off() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _refresh_token, user) = issue_token_pair(&setup).await;

	let grant_id = grant_id_of(&access_token);

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeOAuthGrantRequest>::builder()
				.path(RevokeOAuthGrantPath { grant_id })
				.headers(RevokeOAuthGrantRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(RevokeOAuthGrantRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::ACCEPTED);

	call_api_with(&setup, &access_token)
		.await
		.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_grant_belonging_to_someone_else_cannot_be_revoked() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _refresh_token, _owner) = issue_token_pair(&setup).await;
	let stranger = setup.create_test_user().await;

	let grant_id = grant_id_of(&access_token);

	// Scoped to the caller, so guessing a grant id gets you a 404 rather than
	// somebody else's session.
	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeOAuthGrantRequest>::builder()
				.path(RevokeOAuthGrantPath { grant_id })
				.headers(RevokeOAuthGrantRequestHeaders {
					authorization: stranger.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(RevokeOAuthGrantRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::NOT_FOUND);

	call_api_with(&setup, &access_token)
		.await
		.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn revoking_by_client_ends_every_grant_for_that_app() {
	let setup = setup().await.expect("failed to setup test server");

	// Two grants for the same app — approving a consent screen twice is all
	// it takes, which is exactly why revoking per-app has to exist.
	let (first_code, user) = issue_code(&setup, "openid offline_access").await;
	let first: serde_json::Value = exchange(&setup, &first_code).await.json();
	let first_token = first["access_token"]
		.as_str()
		.expect("an access token")
		.to_owned();

	let second_code = issue_code_for(&setup, &user, "openid offline_access").await;
	let second: serde_json::Value = exchange(&setup, &second_code).await.json();
	let second_token = second["access_token"]
		.as_str()
		.expect("an access token")
		.to_owned();

	assert_ne!(
		grant_id_of(&first_token),
		grant_id_of(&second_token),
		"consenting twice should produce two grants"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeOAuthGrantsForClientRequest>::builder()
				.path(RevokeOAuthGrantsForClientPath {
					client_id: CLIENT_ID.to_owned(),
				})
				.headers(RevokeOAuthGrantsForClientRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(RevokeOAuthGrantsForClientRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::ACCEPTED);

	for token in [&first_token, &second_token] {
		call_api_with(&setup, token)
			.await
			.assert_status(StatusCode::UNAUTHORIZED);
	}
}

#[tokio::test]
async fn grant_management_rejects_an_oauth_token() {
	let setup = setup().await.expect("failed to setup test server");
	let (access_token, _refresh_token, _user) = issue_token_pair(&setup).await;

	// An app enumerating — and revoking — the grants a user has handed out is
	// the same hole as letting it list their browser sessions.
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListOAuthGrantsRequest>::builder()
				.path(ListOAuthGrantsPath)
				.headers(ListOAuthGrantsRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ListOAuthGrantsRequest)
				.build(),
		)
		.await
		.assert_status(StatusCode::FORBIDDEN);
}
