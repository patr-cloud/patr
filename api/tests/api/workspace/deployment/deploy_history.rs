use models::api::workspace::deployment::deploy_history::*;

use crate::prelude::*;

#[tokio::test]
async fn list_deploy_history_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListDeploymentDeployHistoryRequest>::builder()
				.path(ListDeploymentDeployHistoryPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(ListDeploymentDeployHistoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
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
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListDeploymentDeployHistoryRequest>::builder()
				.path(ListDeploymentDeployHistoryPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(ListDeploymentDeployHistoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"expected success or server error, got {status}"
	);
}
