use models::{
	ApiSuccessResponseBody, api::workspace::deployment::deploy_history::*, utils::Uuid,
};

use crate::prelude::*;

/// Insert a row directly into `deployment_deploy_history`. Bypasses the
/// real deploy-on-push flow; sufficient for testing the read/delete/revert
/// endpoints.
async fn seed_deploy_history(
	setup: &TestSetup,
	deployment_id: Uuid,
	repository_id: Uuid,
	image_digest: &str,
	created_offset: &str,
) {
	setup
		.execute_sql(&format!(
			"INSERT INTO deployment_deploy_history \
			 (deployment_id, image_digest, repository_id, created) \
			 VALUES ('{}', '{}', '{}', NOW() - INTERVAL '{}')",
			deployment_id, image_digest, repository_id, created_offset
		))
		.await;
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
		.make_web_dashboard_call(
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
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentDeployHistoryResponse>>();

	assert!(response.response.deploys.is_empty());
}

#[tokio::test]
async fn list_deploy_history_after_multiple_deploys() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let oldest = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
	let middle = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
	let newest = "sha256:3333333333333333333333333333333333333333333333333333333333333333";

	seed_deploy_history(&setup, deployment.id, repo.id, oldest, "3 hours").await;
	seed_deploy_history(&setup, deployment.id, repo.id, middle, "2 hours").await;
	seed_deploy_history(&setup, deployment.id, repo.id, newest, "1 hour").await;

	let response = setup
		.make_web_dashboard_call(
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
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentDeployHistoryResponse>>();

	let digests: Vec<_> = response
		.response
		.deploys
		.iter()
		.map(|d| d.image_digest.as_str())
		.collect();

	assert_eq!(digests, vec![newest, middle, oldest]);
}

#[tokio::test]
async fn delete_deploy_history_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
	seed_deploy_history(&setup, deployment.id, repo.id, digest, "1 hour").await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentDeployHistoryRequest>::builder()
				.path(DeleteDeploymentDeployHistoryPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
					image_digest: digest.to_string(),
				})
				.headers(DeleteDeploymentDeployHistoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteDeploymentDeployHistoryResponse,
		));

	let after = setup
		.make_web_dashboard_call(
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
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentDeployHistoryResponse>>();

	assert!(after.response.deploys.is_empty());
}

#[tokio::test]
async fn delete_deploy_history_nonexistent() {
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
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentDeployHistoryRequest>::builder()
				.path(DeleteDeploymentDeployHistoryPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
					image_digest:
						"sha256:0000000000000000000000000000000000000000000000000000000000000000"
							.to_string(),
				})
				.headers(DeleteDeploymentDeployHistoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleting an unknown deploy history row should fail"
	);
}

#[tokio::test]
async fn revert_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let target = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
	seed_deploy_history(&setup, deployment.id, repo.id, target, "30 minutes").await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevertDeploymentRequest>::builder()
				.path(RevertDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
					image_digest: target.to_string(),
				})
				.headers(RevertDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevertDeploymentResponse));
}

#[tokio::test]
async fn revert_deployment_nonexistent_digest() {
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
		.make_web_dashboard_call(
			ApiRequest::<RevertDeploymentRequest>::builder()
				.path(RevertDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
					image_digest:
						"sha256:0000000000000000000000000000000000000000000000000000000000000000"
							.to_string(),
				})
				.headers(RevertDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"reverting to an unknown digest should fail"
	);
}

#[tokio::test]
async fn revert_deployment_to_current() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let digest = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
	seed_deploy_history(&setup, deployment.id, repo.id, digest, "10 minutes").await;
	setup
		.execute_sql(&format!(
			"UPDATE deployment SET current_live_digest = '{}' WHERE id = '{}'",
			digest, deployment.id
		))
		.await;

	// Handler unconditionally re-applies the digest, so reverting to the
	// current one is a no-op success.
	setup
		.make_web_dashboard_call(
			ApiRequest::<RevertDeploymentRequest>::builder()
				.path(RevertDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
					image_digest: digest.to_string(),
				})
				.headers(RevertDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevertDeploymentResponse));
}
