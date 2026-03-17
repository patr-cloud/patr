use std::collections::BTreeMap;

use models::rbac::WorkspacePermission;

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

#[tokio::test]
async fn loki_push_no_execute_permission_returns_403() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	// Create an API token for the admin but with only Member permissions
	// (no Runner::Execute). The admin can create such a token because they
	// own the workspace.
	let api_token = setup
		.create_test_api_token(
			&admin.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::new(),
				},
			)]),
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
		StatusCode::FORBIDDEN,
		"expected 403 without Runner::Execute permission"
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

	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
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
					&basic_auth(&runner.id, &api_token.token),
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

	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
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
					&basic_auth(&runner.id, &api_token.token),
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

#[tokio::test]
async fn loki_push_wrong_deployment_returns_403() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	// Create two runners, each with a deployment
	let runner_a = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let runner_b = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment_b = setup
		.create_test_deployment(&user.access_token, workspace.id, runner_b.id)
		.await;

	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	// Runner A tries to push logs with runner B's deployment_id
	let labels = format!(
		r#"{{job="test", runner_id="{}", workspace_id="{}", deployment_id="{}"}}"#,
		runner_a.id, workspace.id, deployment_b.id,
	);
	let body = make_loki_push_body(&labels, &["should be rejected"]);

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/loki/api/v1/push",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner_a.id, &api_token.token),
				),
			],
			body,
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::FORBIDDEN,
		"expected 403 when deployment doesn't belong to runner"
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

	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
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
					&basic_auth(&runner.id, &api_token.token),
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

	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
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
					&basic_auth(&runner.id, &api_token.token),
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
