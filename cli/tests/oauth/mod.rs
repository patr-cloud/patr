//! The OAuth session `patr login` produces, and the renewal that keeps it
//! alive.
//!
//! The browser half of the flow can't run here — there is no browser, and the
//! consent screen is the API's. What these cover is everything after it: the
//! stored session, when it gets refreshed, and what happens when the grant
//! behind it is gone.

use cli::prelude::*;
use models::api::{user::*, workspace::Workspace};
use time::OffsetDateTime;
use wiremock::{
	Mock,
	MockServer,
	ResponseTemplate,
	matchers::{body_string_contains, header, method, path},
};

use crate::setup;

/// A `/token` response carrying a fresh pair.
fn token_response(access_token: &str, refresh_token: &str) -> ResponseTemplate {
	ResponseTemplate::new(200).set_body_json(serde_json::json!({
		"access_token": access_token,
		"token_type": "Bearer",
		"expires_in": 3600,
		"refresh_token": refresh_token,
		"scope": "openid profile email offline_access",
	}))
}

/// Mount `GET /user`, but only for a request bearing exactly `token`.
///
/// The narrowness is the assertion: a request carrying anything else matches
/// no mock, comes back a 404, and fails the test.
async fn mount_user_info(server: &MockServer, token: &str) {
	Mock::given(method("GET"))
		.and(path("/user"))
		.and(header("authorization", format!("Bearer {token}").as_str()))
		.respond_with(setup::success(GetUserInfoResponse {
			basic_user_info: WithId::new(
				Uuid::parse_str("00000000000000000000000000000001").unwrap(),
				BasicUserInfo {
					first_name: "Test".to_owned(),
					last_name: "User".to_owned(),
				},
			),
			created: OffsetDateTime::UNIX_EPOCH,
			email: "test@patr.cloud".to_owned(),
			is_mfa_enabled: false,
		}))
		.mount(server)
		.await;
}

/// How many requests the stub has seen for `/auth/oauth/token`.
async fn token_requests(server: &MockServer) -> usize {
	server
		.received_requests()
		.await
		.unwrap_or_default()
		.iter()
		.filter(|request| request.url.path() == "/auth/oauth/token")
		.count()
}

/// An access token about to expire is renewed before the command runs, and the
/// command then uses the new one.
#[tokio::test]
async fn an_expiring_session_is_refreshed_before_the_command_runs() {
	let server = setup::reset().await;

	Mock::given(method("POST"))
		.and(path("/auth/oauth/token"))
		.and(body_string_contains("grant_type=refresh_token"))
		.and(body_string_contains("client_id=patr-cli"))
		.and(body_string_contains("refresh_token=old-refresh"))
		.respond_with(token_response("new-access", "new-refresh"))
		.mount(server)
		.await;
	mount_user_info(server, "new-access").await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() + 10;
	let state = setup::oauth_state("old-access", "old-refresh", expiry);

	setup::run(state, &["patr", "info"])
		.await
		.expect("`patr info` should have succeeded against the refreshed token");

	assert_eq!(token_requests(server).await, 1);

	// The rotated pair has to be written back, and this is the assertion that
	// says so. Saving the old one would make the *next* invocation replay a
	// consumed token — which lands in the grace window once, and revokes the
	// grant the time after.
	let saved = setup::saved_state();
	let AuthState::LoggedIn {
		token,
		current_workspace,
		refresh_token,
		token_expiry,
	} = saved.auth
	else {
		panic!("the refresh logged the user out");
	};

	assert_eq!(token.0.token(), "new-access");
	assert_eq!(refresh_token.as_deref(), Some("new-refresh"));
	assert!(
		token_expiry.is_some_and(|renewed| renewed > expiry),
		"the expiry should have moved forward, got {token_expiry:?}"
	);
	// The session is rebuilt around the new pair, so the rest of it has to
	// survive that.
	assert_eq!(
		current_workspace,
		Some(Uuid::parse_str(setup::STARTING_WORKSPACE).unwrap()),
	);
}

/// A session with plenty of life left is left alone. Refreshing on every
/// invocation would rotate the token needlessly and add a round trip to every
/// command.
#[tokio::test]
async fn a_live_session_is_not_refreshed() {
	let server = setup::reset().await;

	mount_user_info(server, "live-access").await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() + 3600;
	let state = setup::oauth_state("live-access", "live-refresh", expiry);

	setup::run(state, &["patr", "info"])
		.await
		.expect("`patr info` should have succeeded on the existing token");

	assert_eq!(token_requests(server).await, 0);
}

/// A refresh token the server rejects means the grant is gone. The local
/// session is cleared and the user is told to log in again, rather than the
/// command running on and failing with a 401.
#[tokio::test]
async fn a_rejected_refresh_token_logs_the_user_out() {
	let server = setup::reset().await;

	Mock::given(method("POST"))
		.and(path("/auth/oauth/token"))
		.respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
			"error": "invalid_grant",
			"error_description": "the refresh token is not valid",
		})))
		.mount(server)
		.await;
	// Mounted so that a command which wrongly carried on would succeed, and
	// the test would notice.
	mount_user_info(server, "stale-access").await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() - 10;
	let state = setup::oauth_state("stale-access", "dead-refresh", expiry);

	let error = setup::run(state, &["patr", "info"])
		.await
		.expect_err("a rejected refresh should not let the command run");

	assert!(matches!(error, AppError::NotLoggedIn), "got {error:?}");
}

/// An API token session has no refresh token, and nothing to refresh it with.
#[tokio::test]
async fn an_api_token_session_is_never_refreshed() {
	let server = setup::reset().await;

	mount_user_info(server, "patrv1.test-token").await;

	let state = setup::state(Uuid::parse_str("00000000000000000000000000000001").unwrap());

	setup::run(state, &["patr", "info"])
		.await
		.expect("`patr info` should have succeeded on the API token");

	assert_eq!(token_requests(server).await, 0);
}

/// `--token` is the CI path. It must not touch the stored session, even when
/// that session is expired and would otherwise be refreshed.
#[tokio::test]
async fn passing_a_token_skips_the_refresh() {
	let server = setup::reset().await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() - 10;
	let state = setup::oauth_state("stale-access", "stale-refresh", expiry);

	setup::run(state, &["patr", "--token", "patrv1.ci-token", "info"])
		.await
		.expect("`patr info --token` should not have failed");

	assert_eq!(token_requests(server).await, 0);
}

/// `patr workspace switch` rebuilds the whole session, so it has to carry the
/// OAuth half over — dropping it would quietly downgrade a browser login to
/// one that can never refresh.
#[tokio::test]
async fn switching_workspaces_keeps_the_oauth_session() {
	let server = setup::reset().await;

	let workspace_id = Uuid::parse_str("00000000000000000000000000000002").unwrap();

	Mock::given(method("GET"))
		.and(path("/user/workspaces"))
		.respond_with(setup::success(ListUserWorkspacesResponse {
			workspaces: vec![WithId::new(
				workspace_id,
				Workspace {
					name: "test-workspace".to_owned(),
					super_admin_id: Uuid::parse_str("00000000000000000000000000000003").unwrap(),
				},
			)],
		}))
		.mount(server)
		.await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() + 3600;
	let state = setup::oauth_state("live-access", "live-refresh", expiry);

	setup::run(
		state,
		&["patr", "workspace", "switch", "-w", "test-workspace"],
	)
	.await
	.expect("`patr workspace switch` should have succeeded");

	// Read straight off disk rather than through `AppState::load`, which
	// layers the real user config over `CONFIG_PATH` and would answer with the
	// developer's own login.
	let saved = setup::saved_state();
	let AuthState::LoggedIn {
		current_workspace,
		refresh_token,
		token_expiry,
		..
	} = saved.auth
	else {
		panic!("the switch logged the user out");
	};

	assert_eq!(current_workspace, Some(workspace_id));
	assert_eq!(refresh_token.as_deref(), Some("live-refresh"));
	assert_eq!(token_expiry, Some(expiry));
}

/// The loopback callback. These are the checks that stand between a browser
/// request landing on the CLI's port and a code being exchanged.
mod callback {
	use cli::utils::oauth::{CallbackOutcome, interpret_callback};

	/// The happy path: the browser comes back with a code and the state we
	/// sent it.
	#[test]
	fn a_matching_callback_yields_its_code() {
		assert_eq!(
			interpret_callback("GET /callback?code=the-code&state=abc HTTP/1.1\r\n", "abc"),
			CallbackOutcome::Code("the-code".to_owned()),
		);
	}

	/// A callback carrying somebody else's `state` is somebody else's login,
	/// and its code must not be exchanged.
	#[test]
	fn a_mismatched_state_is_refused() {
		let outcome =
			interpret_callback("GET /callback?code=the-code&state=nope HTTP/1.1\r\n", "abc");

		assert!(matches!(outcome, CallbackOutcome::Failed(_)), "{outcome:?}");
	}

	/// A callback with no `state` at all is refused just the same — an
	/// attacker who omits the parameter must not get a free pass.
	#[test]
	fn a_missing_state_is_refused() {
		let outcome = interpret_callback("GET /callback?code=the-code HTTP/1.1\r\n", "abc");

		assert!(matches!(outcome, CallbackOutcome::Failed(_)), "{outcome:?}");
	}

	/// Browsers ask for things nobody mounted. The server has to keep waiting
	/// rather than treat the first request as the answer.
	#[test]
	fn other_paths_are_ignored() {
		for line in [
			"GET /favicon.ico HTTP/1.1\r\n",
			"GET / HTTP/1.1\r\n",
			// Not the callback, however much it looks like it.
			"GET /callbackery?code=x&state=abc HTTP/1.1\r\n",
			"\r\n",
		] {
			assert_eq!(
				interpret_callback(line, "abc"),
				CallbackOutcome::NotForUs,
				"`{line}` should not have been taken for the callback"
			);
		}
	}

	/// A denied consent comes back as an error, and the description is what
	/// the user gets told.
	#[test]
	fn a_denied_consent_carries_its_reason() {
		assert_eq!(
			interpret_callback(
				"GET /callback?error=access_denied&error_description=you+said+no&state=abc \
				 HTTP/1.1\r\n",
				"abc",
			),
			CallbackOutcome::Failed("you said no".to_owned()),
		);
	}
}

/// The token type the session is stored as.
mod storage {
	use std::str::FromStr;

	use models::prelude::BearerToken;

	/// Every other test uses a short made-up token. A real OAuth access token
	/// is a three-segment ES256 JWT, several hundred characters of base64url,
	/// so the stored type has to take one.
	#[test]
	fn a_real_access_token_parses() {
		let jwt = format!(
			"eyJhbGciOiJFUzI1NiIsInR5cCI6ImF0K2p3dCIsImtpZCI6IntrfSJ9.{}.{}",
			"a".repeat(400),
			"b_-9".repeat(22),
		);

		BearerToken::from_str(&jwt).expect("a JWT access token should parse as a bearer token");
	}
}

/// When a stored access token is considered too close to expiry to use.
mod expiry {
	use cli::utils::oauth::needs_refresh;

	/// The margin covers a slow command and clock skew, so a token expiring
	/// within it is renewed even though it is technically still valid.
	#[test]
	fn a_token_inside_the_margin_is_renewed() {
		assert!(needs_refresh(Some(1_000), 1_000));
		assert!(needs_refresh(Some(1_000), 990));
		assert!(!needs_refresh(Some(1_000), 900));
	}

	/// A session stored before the expiry field existed has no recorded
	/// expiry. Refreshing costs one round trip; assuming it is live costs the
	/// user a failed command.
	#[test]
	fn an_unknown_expiry_is_renewed() {
		assert!(needs_refresh(None, 0));
	}
}

/// A server having a bad minute is not a dead grant. Clearing the session over
/// a rate limit would turn a blip into a re-login.
#[tokio::test]
async fn a_rate_limited_refresh_keeps_the_session() {
	let server = setup::reset().await;

	Mock::given(method("POST"))
		.and(path("/auth/oauth/token"))
		.respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
			"error": "temporarily_unavailable",
			"error_description": "too many requests; slow down and try again",
		})))
		.mount(server)
		.await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() - 10;
	let state = setup::oauth_state("stale-access", "live-refresh", expiry);

	let error = setup::run(state, &["patr", "info"])
		.await
		.expect_err("a rate-limited refresh should fail the command");

	assert!(
		matches!(error, AppError::OAuthError(_)),
		"a rate limit should not have logged the user out, got {error:?}"
	);
}
