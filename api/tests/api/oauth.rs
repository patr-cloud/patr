use std::collections::BTreeMap;

use api::{models::oauth::types::OAuthAuthorizationRequest, redis::keys as redis_keys};
use axum_test::TestResponse;
use models::{ApiSuccessResponseBody, api::auth::oauth::*, utils::Uuid};

use crate::prelude::*;

/// The confidential client seeded in `setup.rs`.
const CLIENT_ID: &str = "test-client";
/// Its one registered redirect URI.
const REDIRECT_URI: &str = "http://localhost:19999/cb";
/// A syntactically valid S256 challenge. The exchange half is PR 4's
/// problem; `/authorize` only checks it is present and non-empty.
const CODE_CHALLENGE: &str = "E9Melhoa2OwvfrEr_gYNIYq0M3vLU7QAOPqPzYGZjnI";

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
