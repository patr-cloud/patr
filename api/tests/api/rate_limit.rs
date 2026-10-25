use std::{
	net::IpAddr,
	time::{SystemTime, UNIX_EPOCH},
};

use api::redis::keys;
use models::api::user::*;
use rand::RngExt as _;
use rustis::commands::{SortedSetCommands as _, ZAddOptions};

use crate::prelude::*;

/// The debug-build per-second window from the API rate limiter
/// (`api/src/utils/layers/rate_limiter_layer.rs`). Debug limits are looser
/// than production (SPA page-load bursts tripped 20/s in the e2e suite), so
/// the tests fill this value instead.
const DEBUG_PER_SECOND_LIMIT: usize = 50;

/// Each rate-limit test pins requests to its own randomly-chosen IPv4 so the
/// per-IP bucket accumulates predictably even when Redis is shared across
/// tests. Collisions across concurrent tests have negligible probability.
fn fixed_test_ip() -> IpAddr {
	IpAddr::V4(rand::rng().random::<u32>().into())
}

/// Fills the per-second window of a session's per-login bucket, as if it had
/// just made `DEBUG_PER_SECOND_LIMIT` requests. The entries are written
/// straight to the limiter's sorted set rather than sent as requests, so the
/// window is full however slowly the runner serves them.
async fn fill_per_second_limit(setup: &TestSetup, refresh_token: &BearerToken) {
	let login_id = Uuid::parse_str(refresh_token.0.token().split_once('.').unwrap().0).unwrap();
	let now_ns = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_nanos() as f64;

	setup
		.state()
		.redis
		.zadd(
			keys::rate_limit_login_id(&login_id, 1),
			(0..DEBUG_PER_SECOND_LIMIT)
				.map(|i| (now_ns, format!("{now_ns}-{i}")))
				.collect::<Vec<_>>(),
			ZAddOptions::default(),
		)
		.await
		.expect("failed to fill the per-second window");
}

#[tokio::test]
async fn test_rate_limit_allows_requests_under_limit() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ip = fixed_test_ip();

	// Make 2 requests (well under the per-second limit). Both should succeed.
	for _ in 0..2 {
		let response = setup
			.make_web_dashboard_call_from_ip(
				ApiRequest::<GetUserInfoRequest>::builder()
					.headers(GetUserInfoRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
				ip,
			)
			.await;

		assert_eq!(
			response.status_code(),
			StatusCode::OK,
			"expected 200 OK for requests under rate limit"
		);
	}
}

#[tokio::test]
async fn test_rate_limit_blocks_after_exceeding_per_second_limit() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ip = fixed_test_ip();

	// Fill the per-second window, then send one more.
	fill_per_second_limit(&setup, &user.refresh_token).await;

	// The next request should be rate-limited
	let response = setup
		.make_web_dashboard_call_from_ip(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			ip,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::TOO_MANY_REQUESTS,
		"expected 429 after exceeding per-second rate limit"
	);
}

#[tokio::test]
async fn test_rate_limit_window_slides() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ip = fixed_test_ip();

	// Fill the per-second window
	fill_per_second_limit(&setup, &user.refresh_token).await;

	// Confirm we're rate-limited
	let response = setup
		.make_web_dashboard_call_from_ip(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			ip,
		)
		.await;
	assert_eq!(response.status_code(), StatusCode::TOO_MANY_REQUESTS);

	// Wait for the 1-second window to slide
	tokio::time::sleep(std::time::Duration::from_secs(2)).await;

	// Should be allowed again
	let response = setup
		.make_web_dashboard_call_from_ip(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			ip,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::OK,
		"expected 200 OK after window slides"
	);
}

#[tokio::test]
async fn test_rate_limit_rejected_requests_count() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ip = fixed_test_ip();

	// Fill the per-second window
	fill_per_second_limit(&setup, &user.refresh_token).await;

	// Send more requests — they should all be 429 because rejected requests
	// also consume a slot in the sorted set (optimistic add)
	for _ in 0..3 {
		let response = setup
			.make_web_dashboard_call_from_ip(
				ApiRequest::<GetUserInfoRequest>::builder()
					.headers(GetUserInfoRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
				ip,
			)
			.await;

		assert_eq!(
			response.status_code(),
			StatusCode::TOO_MANY_REQUESTS,
			"rejected requests should still count against the rate limit"
		);
	}
}

#[tokio::test]
async fn test_rate_limit_authenticated_per_login() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ip = fixed_test_ip();

	// Log in again to get a second session (different login_id, same user)
	let (session_b_token, _) = setup.login_test_user(&user.email, &user.password).await;
	let session_b_bearer = BearerToken::from_str(&session_b_token).unwrap();

	// Fill the per-second window of the first session
	fill_per_second_limit(&setup, &user.refresh_token).await;

	// First session should be rate-limited
	let response = setup
		.make_web_dashboard_call_from_ip(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			ip,
		)
		.await;
	assert_eq!(
		response.status_code(),
		StatusCode::TOO_MANY_REQUESTS,
		"session A should be rate-limited"
	);

	// Session B has a different login_id, so its per-login bucket is separate.
	// Authenticated callers are evaluated against their per-login bucket only
	// (per-IP is skipped to avoid penalising users behind shared NAT/CGNAT),
	// so session B should be allowed through even from the same IP.
	let response = setup
		.make_web_dashboard_call_from_ip(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: session_b_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			ip,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::OK,
		"session B has its own per-login bucket and should not be blocked"
	);
}
