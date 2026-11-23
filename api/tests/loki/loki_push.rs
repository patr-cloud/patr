use std::collections::BTreeMap;

use models::{
	api::workspace::deployment::{
		GetDeploymentInfoPath,
		GetDeploymentInfoRequest,
		GetDeploymentInfoRequestHeaders,
		GetDeploymentInfoResponse,
		UpdateDeploymentPath,
		UpdateDeploymentRequest,
		UpdateDeploymentRequestHeaders,
	},
	rbac::WorkspacePermission,
	utils::Uuid,
};

use super::helpers::*;
use crate::prelude::*;

#[tokio::test]
async fn loki_push_no_auth_returns_401() {
	let setup = setup().await.expect("failed to setup test server");

	let body = make_loki_push_body(r#"{job="test"}"#, &["hello"]);
	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![(http::header::CONTENT_TYPE, "application/x-protobuf")],
			body,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"expected 401 without Authorization header"
	);
}

#[tokio::test]
async fn loki_push_invalid_token_returns_401() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let body = make_loki_push_body(r#"{job="test"}"#, &["hello"]);
	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, "invalid-token-value"),
				),
			],
			body,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"expected 401 with invalid API token"
	);
}

/// A runner's token pushes as that runner only, never as another runner in
/// the same workspace.
#[tokio::test]
async fn loki_push_as_another_runner_returns_403() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let other_runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let body = make_loki_push_body(r#"{job="test"}"#, &["hello"]);
	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&other_runner.id, &runner.token),
				),
			],
			body,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::FORBIDDEN,
		"expected 403 for a runner pushing as another runner"
	);
}

/// Only a runner's own token may push. A service account fails
/// authentication outright, whatever it holds.
#[tokio::test]
async fn loki_push_service_account_returns_401() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let service_account = setup
		.create_test_service_account(&admin.access_token, workspace.id, Vec::new())
		.await;

	let body = make_loki_push_body(r#"{job="test"}"#, &["hello"]);
	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &service_account.token),
				),
			],
			body,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"log push must be rejected for a service account"
	);
}

#[tokio::test]
async fn loki_push_user_api_token_returns_401() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	// A workspace super-admin API token has every permission, but log push is
	// restricted to runners — a human credential must not be able to write
	// logs on a runner's behalf. The route only authenticates runners, so an
	// API token fails authentication outright.
	let api_token = setup
		.create_test_api_token(
			&admin.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let body = make_loki_push_body(r#"{job="test"}"#, &["hello"]);
	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &api_token.token),
				),
			],
			body,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"log push must be rejected for non-runner clients"
	);
}

/// An unknown runner id is refused exactly like a runner the token can't
/// execute, so a valid token can't be used to probe which runner ids exist.
#[tokio::test]
async fn loki_push_unknown_runner_matches_missing_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let other_runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	// `runner`'s token may act as `runner` only, so `other_runner` exists but
	// is denied.
	let mut responses = Vec::new();
	for runner_id in [other_runner.id, Uuid::new_v4()] {
		let response = setup
			.make_loki_call(
				http::Method::POST,
				"/loki/api/v1/push",
				vec![
					(http::header::CONTENT_TYPE, "application/x-protobuf"),
					(
						http::header::AUTHORIZATION,
						&basic_auth(&runner_id, &runner.token),
					),
				],
				make_loki_push_body(r#"{job="test"}"#, &["hello"]),
			)
			.await;
		responses.push((response.status_code(), response.text()));
	}

	assert_eq!(responses[0].0, StatusCode::FORBIDDEN);
	assert_eq!(
		responses[0], responses[1],
		"an unknown runner must be refused exactly like a denied one"
	);
}

#[tokio::test]
async fn loki_push_valid_auth_succeeds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let labels = format!(
		r#"{{job="test", runner_id="{}", workspace_id="{}", deployment_id="{}"}}"#,
		runner.id, workspace.id, deployment.id,
	);
	let body = make_loki_push_body(&labels, &["test log line for valid push"]);

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &runner.token),
				),
			],
			body,
		)
		.await;

	let status = response.status_code();
	assert!(
		status == StatusCode::NO_CONTENT || status == StatusCode::OK,
		"expected 204 or 200 from Loki, got {status}"
	);
}

#[tokio::test]
async fn loki_push_label_rewriting() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	// Push with spoofed runner_id and workspace_id — the server should rewrite
	// them to the actual values derived from auth.
	let spoofed_runner = Uuid::new_v4();
	let spoofed_workspace = Uuid::new_v4();
	let labels = format!(
		r#"{{job="rewrite_test", runner_id="{}", workspace_id="{}", deployment_id="{}"}}"#,
		spoofed_runner, spoofed_workspace, deployment.id,
	);
	let body = make_loki_push_body(&labels, &["rewrite-verification-line"]);

	let push_response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &runner.token),
				),
			],
			body,
		)
		.await;

	let status = push_response.status_code();
	assert!(
		status == StatusCode::NO_CONTENT || status == StatusCode::OK,
		"push should succeed, got {status}"
	);

	// Wait a moment for Loki to index the log
	tokio::time::sleep(std::time::Duration::from_secs(2)).await;

	// Query Loki directly for the real runner_id label
	let query_url = format!("{}/loki/api/v1/query_range", setup.upstream_loki_url());
	let query_response = reqwest::Client::new()
		.get(&query_url)
		.query(&[
			("query", format!(r#"{{runner_id="{}"}}"#, runner.id)),
			("limit", "10".to_string()),
		])
		.header("X-Scope-OrgID", workspace.id.to_string())
		.send()
		.await
		.expect("failed to query Loki");

	let body_text = query_response.text().await.unwrap();
	assert!(
		body_text.contains("rewrite-verification-line"),
		"expected to find the pushed log line under the real runner_id label, got: {body_text}"
	);
}

/// Push one Loki stream for a deployment as a runner, returning the status.
async fn push_line(
	setup: &TestSetup,
	runner_id: Uuid,
	token: &str,
	deployment_id: Uuid,
	line: &str,
) -> StatusCode {
	setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(http::header::AUTHORIZATION, &basic_auth(&runner_id, token)),
			],
			make_loki_push_body(
				&format!(r#"{{job="test", deployment_id="{deployment_id}"}}"#),
				&[line],
			),
		)
		.await
		.status_code()
}

/// A batch whose only stream names another runner's deployment is dropped
/// whole: nothing reaches Loki, and the runner gets Loki's success status so it
/// doesn't retry.
#[tokio::test]
async fn loki_push_wrong_deployment_is_dropped() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let runner_a = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let runner_b = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment_b = setup
		.create_test_deployment(&user.access_token, workspace.id, runner_b.id)
		.await;

	let dropped = format!("dropped-{}", random_name(8));
	assert_eq!(
		push_line(
			&setup,
			runner_a.id,
			&runner_a.token,
			deployment_b.id,
			&dropped
		)
		.await,
		StatusCode::NO_CONTENT,
		"a push with nothing left to forward should still succeed"
	);

	// The deployment's own runner pushes after, so once its line is in Loki the
	// dropped one would be too if it had been forwarded.
	let fence = format!("fence-{}", random_name(8));
	assert_eq!(
		push_line(
			&setup,
			runner_b.id,
			&runner_b.token,
			deployment_b.id,
			&fence
		)
		.await,
		StatusCode::NO_CONTENT
	);
	let logs = wait_for_loki_line(&setup, workspace.id, &fence).await;
	assert!(logs.contains(&fence), "the allowed push should reach Loki");
	assert!(
		!logs.contains(&dropped),
		"another runner's stream must not reach Loki"
	);
}

/// In a mixed batch, only the streams the runner may push reach Loki.
#[tokio::test]
async fn loki_push_forwards_only_allowed_streams() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let other_runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let other_deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, other_runner.id)
		.await;

	let own = format!("own-{}", random_name(8));
	let runner_level = format!("runner-{}", random_name(8));
	let foreign = format!("foreign-{}", random_name(8));
	let malformed = format!("malformed-{}", random_name(8));
	let body = make_loki_push_body_streams(&[
		(
			&format!(r#"{{job="test", deployment_id="{}"}}"#, deployment.id),
			&own,
		),
		(r#"{job="test"}"#, &runner_level),
		(
			&format!(r#"{{job="test", deployment_id="{}"}}"#, other_deployment.id),
			&foreign,
		),
		(r#"{job="test", deployment_id="not-a-uuid"}"#, &malformed),
	]);

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &runner.token),
				),
			],
			body,
		)
		.await;
	assert_eq!(
		response.status_code(),
		StatusCode::NO_CONTENT,
		"the allowed streams should be forwarded"
	);

	let logs = wait_for_loki_line(&setup, workspace.id, &own).await;
	assert!(
		logs.contains(&own),
		"the runner's own deployment's stream should reach Loki"
	);
	assert!(
		logs.contains(&runner_level),
		"the runner's own stream should reach Loki"
	);
	assert!(
		!logs.contains(&foreign),
		"another runner's deployment's stream must not reach Loki"
	);
	assert!(
		!logs.contains(&malformed),
		"a stream with a malformed deployment_id must not reach Loki"
	);
}

/// After a move, a late push from the old runner is dropped, and the new
/// runner's goes through.
#[tokio::test]
async fn loki_push_after_move_follows_the_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let old_runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let new_runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, old_runner.id)
		.await;

	// Warm the proxies' cache of which runner the deployment is on.
	let before = format!("before-{}", random_name(8));
	assert_eq!(
		push_line(
			&setup,
			old_runner.id,
			&old_runner.token,
			deployment.id,
			&before
		)
		.await,
		StatusCode::NO_CONTENT
	);

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>()
		.response;
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateDeploymentRequest>::builder()
				.path(UpdateDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(UpdateDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateDeploymentRequest {
					name: info.deployment.name.clone(),
					image_tag: info.deployment.image_tag.clone(),
					runner: new_runner.id,
					machine_type: info.deployment.machine_type,
					running_details: info.running_details.clone(),
				})
				.build(),
		)
		.await
		.assert_status_success();

	let late = format!("late-{}", random_name(8));
	assert_eq!(
		push_line(
			&setup,
			old_runner.id,
			&old_runner.token,
			deployment.id,
			&late
		)
		.await,
		StatusCode::NO_CONTENT,
		"the old runner's late push should be dropped, not refused"
	);
	let after = format!("after-{}", random_name(8));
	assert_eq!(
		push_line(
			&setup,
			new_runner.id,
			&new_runner.token,
			deployment.id,
			&after
		)
		.await,
		StatusCode::NO_CONTENT
	);

	let logs = wait_for_loki_line(&setup, workspace.id, &after).await;
	assert!(
		logs.contains(&after),
		"the new runner's push should reach Loki"
	);
	assert!(
		!logs.contains(&late),
		"the old runner's push after the move must not reach Loki"
	);
}

#[tokio::test]
async fn loki_push_invalid_snappy_returns_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	// Send garbage bytes (not valid snappy)
	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &runner.token),
				),
			],
			vec![0xDE, 0xAD, 0xBE, 0xEF],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::BAD_REQUEST,
		"expected 400 for invalid snappy data"
	);
}

#[tokio::test]
async fn loki_push_invalid_protobuf_returns_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	// Valid snappy of garbage (not valid protobuf)
	let garbage = vec![0x01, 0x02, 0x03, 0x04, 0x05];
	let compressed = snap::raw::Encoder::new()
		.compress_vec(&garbage)
		.expect("snappy compress failed");

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &runner.token),
				),
			],
			compressed,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::BAD_REQUEST,
		"expected 400 for invalid protobuf inside valid snappy"
	);
}
