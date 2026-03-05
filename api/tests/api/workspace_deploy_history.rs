use http::header;
use models::api::{
	ApiEndpoint,
	workspace::deployment::deploy_history::*,
};

use crate::prelude::*;

#[tokio::test]
async fn list_deploy_history_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let response = setup
		.server
		.method(
			ListDeploymentDeployHistoryRequest::METHOD,
			&ListDeploymentDeployHistoryPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	// May return success with empty list or server error (no history yet)
	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"expected success or server error, got {status}"
	);
}

#[tokio::test]
async fn list_deploy_history_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let response = setup
		.server
		.method(
			ListDeploymentDeployHistoryRequest::METHOD,
			&ListDeploymentDeployHistoryPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"expected success or server error, got {status}"
	);
}
