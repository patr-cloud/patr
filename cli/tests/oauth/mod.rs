//! The OAuth session `patr login` leaves behind, and its renewal. The browser
//! half of the flow can't run here, so these cover everything after it.

use std::process::{Output, Stdio};

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

/// The workspace an [`oauth_state`] starts on.
const STARTING_WORKSPACE: &str = "000000000000000000000000000000ff";

/// A logged-in state holding an OAuth session whose access token expires at
/// `token_expiry`.
fn oauth_state(token: &str, refresh_token: &str, token_expiry: i64) -> AppState {
	AppState {
		target_channel: Channel::Alpha,
		auth: AuthState::LoggedIn {
			token: token.parse::<BearerToken>().unwrap(),
			current_workspace: Some(Uuid::parse_str(STARTING_WORKSPACE).unwrap()),
			session: Some(OAuthSession {
				refresh_token: refresh_token.to_owned(),
				token_expiry,
			}),
		},
	}
}

/// Runs `patr` on `state`, in a temp home so the real login is never touched,
/// and hands back its output and the state it left.
async fn run(state: &AppState, args: &[&str]) -> (Output, AppState) {
	let dir = tempfile::tempdir().expect("failed to create a temp dir");
	let state_path = dir.path().join("cli.json");
	std::fs::write(&state_path, serde_json::to_string(state).unwrap())
		.expect("failed to write the CLI state");

	let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_patr"))
		.args(args)
		.env("HOME", dir.path())
		.env("CONFIG_PATH", &state_path)
		.env_remove("XDG_DATA_HOME")
		.env_remove("PATR_TOKEN")
		.stdin(Stdio::null())
		.output()
		.await
		.expect("failed to run patr");

	let saved = serde_json::from_slice(&std::fs::read(&state_path).unwrap())
		.expect("the saved state should deserialize");

	(output, saved)
}

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

/// Mounts `GET /user` for a request bearing exactly `token`, so a request
/// with any other token gets a 404.
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

/// An access token about to expire is renewed before the command runs, which
/// then uses the new one, and the rotated pair is saved.
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
	let (output, saved) = run(&oauth_state("old-access", "old-refresh", expiry), &["info"]).await;

	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert_eq!(token_requests(server).await, 1);

	// Saving the old pair would have the next run replay a consumed token.
	let AuthState::LoggedIn {
		token,
		current_workspace,
		session: Some(session),
	} = saved.auth
	else {
		panic!("the refresh dropped the session: {:?}", saved.auth);
	};
	assert_eq!(token.0.token(), "new-access");
	assert_eq!(session.refresh_token, "new-refresh");
	assert!(session.token_expiry > expiry);
	assert_eq!(
		current_workspace,
		Some(Uuid::parse_str(STARTING_WORKSPACE).unwrap())
	);
}

/// A session with time left isn't refreshed.
#[tokio::test]
async fn a_live_session_is_not_refreshed() {
	let server = setup::reset().await;
	mount_user_info(server, "live-access").await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() + 3600;
	let (output, _saved) = run(
		&oauth_state("live-access", "live-refresh", expiry),
		&["info"],
	)
	.await;

	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert_eq!(token_requests(server).await, 0);
}

/// A refresh token the server rejects means the grant is gone, so the session
/// is cleared and a command that needs it fails.
#[tokio::test]
async fn a_rejected_refresh_logs_the_user_out() {
	let server = setup::reset().await;

	Mock::given(method("POST"))
		.and(path("/auth/oauth/token"))
		.respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
			"error": "invalid_grant",
			"error_description": "the grant has been revoked",
		})))
		.mount(server)
		.await;
	// So a command that wrongly carried on would succeed.
	mount_user_info(server, "stale-access").await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() - 10;
	let (output, saved) = run(
		&oauth_state("stale-access", "dead-refresh", expiry),
		&["info"],
	)
	.await;

	assert!(!output.status.success());
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("not logged in"),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(matches!(saved.auth, AuthState::LoggedOut {}));
}

/// A server having a bad minute isn't a dead grant. The session is kept and
/// the command carries on with the token it has.
#[tokio::test]
async fn a_failed_refresh_keeps_the_session() {
	let server = setup::reset().await;

	Mock::given(method("POST"))
		.and(path("/auth/oauth/token"))
		.respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
			"error": "temporarily_unavailable",
			"error_description": "too many requests; slow down and try again",
		})))
		.mount(server)
		.await;
	mount_user_info(server, "old-access").await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() + 10;
	let state = oauth_state("old-access", "live-refresh", expiry);
	let (output, saved) = run(&state, &["info"]).await;

	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(String::from_utf8_lossy(&output.stderr).contains("Couldn't renew your login"));
	assert_eq!(
		serde_json::to_value(&saved.auth).unwrap(),
		serde_json::to_value(&state.auth).unwrap()
	);
}

/// An API token has nothing to refresh.
#[tokio::test]
async fn an_api_token_session_is_never_refreshed() {
	let server = setup::reset().await;
	mount_user_info(server, "patrv1.test-token").await;

	let (output, _saved) = run(
		&setup::state(Uuid::parse_str(STARTING_WORKSPACE).unwrap()),
		&["info"],
	)
	.await;

	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert_eq!(token_requests(server).await, 0);
}

/// `--token` leaves the stored session alone, even an expired one.
#[tokio::test]
async fn passing_a_token_skips_the_refresh() {
	let server = setup::reset().await;
	mount_user_info(server, "patrv1.ci-token").await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() - 10;
	let (output, _saved) = run(
		&oauth_state("stale-access", "stale-refresh", expiry),
		&["--token", "patrv1.ci-token", "info"],
	)
	.await;

	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert_eq!(token_requests(server).await, 0);
}

/// `patr workspace switch` rebuilds the login, and has to keep its session.
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
	let (output, saved) = run(
		&oauth_state("live-access", "live-refresh", expiry),
		&["workspace", "switch", "-w", "test-workspace"],
	)
	.await;

	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	let AuthState::LoggedIn {
		current_workspace,
		session,
		..
	} = saved.auth
	else {
		panic!("the switch logged the user out");
	};
	assert_eq!(current_workspace, Some(workspace_id));
	assert_eq!(
		session,
		Some(OAuthSession {
			refresh_token: "live-refresh".to_owned(),
			token_expiry: expiry,
		})
	);
}

/// `patr logout` revokes the grant behind the session.
#[tokio::test]
async fn logging_out_revokes_the_grant() {
	let server = setup::reset().await;

	Mock::given(method("POST"))
		.and(path("/auth/oauth/revoke"))
		.and(body_string_contains("token=live-refresh"))
		.and(body_string_contains("client_id=patr-cli"))
		.respond_with(ResponseTemplate::new(200))
		.expect(1)
		.mount(server)
		.await;

	let expiry = OffsetDateTime::now_utc().unix_timestamp() + 3600;
	let (output, saved) = run(
		&oauth_state("live-access", "live-refresh", expiry),
		&["logout"],
	)
	.await;

	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(!String::from_utf8_lossy(&output.stderr).contains("Couldn't revoke"));
	assert!(matches!(saved.auth, AuthState::LoggedOut {}));
	server.verify().await;
}

/// The checks between a request landing on the loopback port and a code being
/// exchanged.
mod callback {
	use cli::utils::oauth::{CallbackOutcome, interpret_callback};

	#[test]
	fn a_matching_callback_yields_its_code() {
		assert_eq!(
			interpret_callback("GET /callback?code=the-code&state=abc HTTP/1.1\r\n", "abc"),
			CallbackOutcome::Code("the-code".to_owned()),
		);
	}

	#[test]
	fn a_mismatched_or_missing_state_is_refused() {
		for line in [
			"GET /callback?code=the-code&state=nope HTTP/1.1\r\n",
			"GET /callback?code=the-code HTTP/1.1\r\n",
		] {
			let outcome = interpret_callback(line, "abc");
			assert!(matches!(outcome, CallbackOutcome::Failed(_)), "{outcome:?}");
		}
	}

	#[test]
	fn other_paths_are_ignored() {
		for line in [
			"GET /favicon.ico HTTP/1.1\r\n",
			"GET / HTTP/1.1\r\n",
			"GET /callbackery?code=x&state=abc HTTP/1.1\r\n",
			"\r\n",
		] {
			assert_eq!(
				interpret_callback(line, "abc"),
				CallbackOutcome::NotForUs,
				"`{line}` was taken for the callback"
			);
		}
	}

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

/// An OAuth access token is a long ES256 JWT, which the stored token type has
/// to take.
#[test]
fn a_real_access_token_parses() {
	let jwt = format!(
		"eyJhbGciOiJFUzI1NiIsInR5cCI6ImF0K2p3dCIsImtpZCI6IntrfSJ9.{}.{}",
		"a".repeat(400),
		"b_-9".repeat(22),
	);

	jwt.parse::<BearerToken>()
		.expect("a JWT access token should parse as a bearer token");
}
