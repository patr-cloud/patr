use models::api::workspace::{
	container_registry::*,
	deployment::*,
	domain::*,
	runner::*,
	secret::*,
};

use crate::prelude::*;

/// The authorizer only proves an id is some resource in the workspace, so each
/// typed delete has to check it actually is one of its own kind. Passing the id
/// of some other resource (or of the workspace itself) must be refused and must
/// leave that resource alone — otherwise the route soft-deletes whatever it was
/// handed.
#[tokio::test]
async fn typed_deletes_refuse_an_id_of_another_type() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut statuses = Vec::new();
	for secret_id in [deployment.id, runner.id, workspace.id] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<DeleteSecretRequest>::builder()
					.path(DeleteSecretPath {
						workspace_id: workspace.id,
						secret_id,
					})
					.headers(DeleteSecretRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await;
		statuses.push(("secret", response.status_code()));
	}

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDomainInWorkspaceRequest>::builder()
				.path(DeleteDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: deployment.id,
				})
				.headers(DeleteDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	statuses.push(("domain", response.status_code()));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryRequest>::builder()
				.path(DeleteContainerRepositoryPath {
					workspace_id: workspace.id,
					repository_id: deployment.id,
				})
				.headers(DeleteContainerRepositoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	statuses.push(("repository", response.status_code()));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: deployment.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	statuses.push(("runner", response.status_code()));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: runner.id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	statuses.push(("deployment", response.status_code()));

	assert!(
		statuses
			.iter()
			.all(|(_, status)| *status == StatusCode::NOT_FOUND),
		"every wrong-type delete must be refused as not found, got {statuses:?}"
	);

	// Nothing that was handed to the wrong route may have been soft-deleted.
	setup
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
		.assert_status_ok();
	setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();
	// The workspace itself still takes new resources.
	setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
}
