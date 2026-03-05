use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::runner::*,
	},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn add_runner_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	assert!(!runner.name.is_empty());
}

#[tokio::test]
async fn list_runners_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let _runner = create_test_runner(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			ListRunnersForWorkspaceRequest::METHOD,
			&ListRunnersForWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();

	assert_eq!(1, response.response.runners.len());
}

#[tokio::test]
async fn list_runners_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListRunnersForWorkspaceRequest::METHOD,
			&ListRunnersForWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();

	assert!(response.response.runners.is_empty());
}

#[tokio::test]
async fn get_runner_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetRunnerInfoResponse>>();

	assert_eq!(runner.name, response.response.runner.name);
}

#[tokio::test]
async fn get_runner_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent runner"
	);
}

#[tokio::test]
async fn get_ingress_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			GetIngressTokenForRunnerRequest::METHOD,
			&GetIngressTokenForRunnerPath {
				workspace_id: ws.id,
				runner_id: runner.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetIngressTokenForRunnerResponse>>();

	assert!(
		!response.response.token.is_empty(),
		"ingress token should not be empty"
	);
}

#[tokio::test]
async fn remove_runner_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;

	setup
		.server
		.method(
			DeleteRunnerRequest::METHOD,
			&DeleteRunnerPath {
				workspace_id: ws.id,
				runner_id: runner.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRunnerResponse));

	// Verify it's gone
	let response = setup
		.server
		.method(
			GetRunnerInfoRequest::METHOD,
			&GetRunnerInfoPath {
				workspace_id: ws.id,
				runner_id: runner.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleted runner should not be found"
	);
}

#[tokio::test]
async fn remove_runner_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			DeleteRunnerRequest::METHOD,
			&DeleteRunnerPath {
				workspace_id: ws.id,
				runner_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent runner"
	);
}

#[tokio::test]
async fn runner_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListRunnersForWorkspaceRequest::METHOD,
			&ListRunnersForWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}
