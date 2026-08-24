use std::collections::{BTreeMap, BTreeSet};

use models::rbac::WorkspacePermission;

use super::helpers::*;
use crate::prelude::*;

#[tokio::test]
async fn otlp_push_no_auth_returns_401() {
	let setup = setup().await.expect("failed to setup test server");

	let body = make_otlp_json_body(&[("job", "test")]);
	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
			vec![(http::header::CONTENT_TYPE, "application/json")],
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
async fn otlp_push_json_valid_succeeds() {
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
		.create_test_api_token(&user.access_token, BTreeSet::from([workspace.id]), BTreeMap::new())
		.await;

	let body = make_otlp_json_body(&[
		("runner_id", &runner.id.to_string()),
		("workspace_id", &workspace.id.to_string()),
		("deployment_id", &deployment.id.to_string()),
	]);

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
			vec![
				(http::header::CONTENT_TYPE, "application/json"),
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
async fn otlp_push_protobuf_valid_succeeds() {
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
		.create_test_api_token(&user.access_token, BTreeSet::from([workspace.id]), BTreeMap::new())
		.await;

	let body = make_otlp_proto_body(&[
		("runner_id", &runner.id.to_string()),
		("workspace_id", &workspace.id.to_string()),
		("deployment_id", &deployment.id.to_string()),
	]);

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
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
async fn otlp_push_unsupported_content_type_returns_415() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeSet::from([workspace.id]), BTreeMap::new())
		.await;

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
			vec![
				(http::header::CONTENT_TYPE, "text/plain"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &api_token.token),
				),
			],
			b"some text".to_vec(),
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNSUPPORTED_MEDIA_TYPE,
		"expected 415 for unsupported content type"
	);
}

#[tokio::test]
async fn otlp_push_wrong_deployment_returns_403() {
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

	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeSet::from([workspace.id]), BTreeMap::new())
		.await;

	// Runner A tries to push logs with runner B's deployment_id
	let body = make_otlp_json_body(&[
		("runner_id", &runner_a.id.to_string()),
		("workspace_id", &workspace.id.to_string()),
		("deployment_id", &deployment_b.id.to_string()),
	]);

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
			vec![
				(http::header::CONTENT_TYPE, "application/json"),
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
async fn otlp_push_attribute_rewriting() {
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
		.create_test_api_token(&user.access_token, BTreeSet::from([workspace.id]), BTreeMap::new())
		.await;

	// Push with spoofed runner_id and workspace_id
	let spoofed_runner = Uuid::new_v4();
	let spoofed_workspace = Uuid::new_v4();
	let body = make_otlp_json_body(&[
		("runner_id", &spoofed_runner.to_string()),
		("workspace_id", &spoofed_workspace.to_string()),
		("deployment_id", &deployment.id.to_string()),
		("job", "otlp_rewrite_test"),
	]);

	let push_response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
			vec![
				(http::header::CONTENT_TYPE, "application/json"),
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

	// Wait for Loki to index (testcontainers Loki can be slow)
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	// Query Loki for the real runner_id
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
		body_text.contains("test log line from OTLP"),
		"expected OTLP log line under the real runner_id, got: {body_text}"
	);
}

#[tokio::test]
async fn otlp_push_invalid_json_returns_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeSet::from([workspace.id]), BTreeMap::new())
		.await;

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
			vec![
				(http::header::CONTENT_TYPE, "application/json"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &api_token.token),
				),
			],
			b"not valid json{{{".to_vec(),
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::BAD_REQUEST,
		"expected 400 for invalid JSON"
	);
}

#[tokio::test]
async fn otlp_push_invalid_protobuf_returns_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let api_token = setup
		.create_test_api_token(&user.access_token, BTreeSet::from([workspace.id]), BTreeMap::new())
		.await;

	let response = setup
		.make_loki_call(
			http::Method::POST,
			"/otlp/v1/logs",
			vec![
				(http::header::CONTENT_TYPE, "application/x-protobuf"),
				(
					http::header::AUTHORIZATION,
					&basic_auth(&runner.id, &api_token.token),
				),
			],
			vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::BAD_REQUEST,
		"expected 400 for invalid protobuf"
	);
}
