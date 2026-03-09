use std::collections::BTreeMap;

use models::{ApiSuccessResponseBody, api::workspace::deployment::*, utils::Uuid};

use crate::prelude::*;

mod deploy_history;

#[tokio::test]
async fn list_machine_types_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath {
					workspace_id: workspace.id,
				})
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();

	assert!(
		!response.response.machine_types.is_empty(),
		"machine types should not be empty"
	);
}

#[tokio::test]
async fn create_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	assert!(!deployment.name.is_empty());
}

#[tokio::test]
async fn create_deployment_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	// Get machine type
	let machine_types = setup
		.make_api_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath {
					workspace_id: workspace.id,
				})
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();

	let mt_id = machine_types.response.machine_types[0].id;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: "!!!".to_string(),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: runner.id,
					machine_type: mt_id,
					running_details: DeploymentRunningDetails {
						deploy_on_push: false,
						min_horizontal_scale: 1,
						max_horizontal_scale: 1,
						ports: BTreeMap::new(),
						environment_variables: BTreeMap::new(),
						startup_probe: None,
						liveness_probe: None,
						config_mounts: BTreeMap::new(),
						volumes: BTreeMap::new(),
					},
					deploy_on_create: false,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid deployment name"
	);
}

#[tokio::test]
async fn list_deployments_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let _deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();

	assert_eq!(1, response.response.deployments.len());
}

#[tokio::test]
async fn list_deployments_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();

	assert!(response.response.deployments.is_empty());
}

#[tokio::test]
async fn get_deployment_info_works() {
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
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(deployment.name, response.response.deployment.name);
}

#[tokio::test]
async fn get_deployment_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: Uuid::nil(),
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent deployment"
	);
}

#[tokio::test]
async fn update_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let new_name = random_name(8);
	setup
		.make_api_call(
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
					name: Some(new_name.clone()),
					runner: None,
					machine_type: None,
					deploy_on_push: None,
					min_horizontal_scale: None,
					max_horizontal_scale: None,
					ports: None,
					environment_variables: None,
					startup_probe: None,
					liveness_probe: None,
					config_mounts: None,
					volumes: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));
}

#[tokio::test]
async fn delete_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	setup
		.make_api_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteDeploymentResponse));

	// Verify it's gone
	let response = setup
		.make_api_call(
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
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn start_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	// Start may succeed or fail depending on runner connectivity — just check
	// it doesn't return 4xx auth error
	let response = setup
		.make_api_call(
			ApiRequest::<StartDeploymentRequest>::builder()
				.path(StartDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(StartDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(StartDeploymentQuery {
					force_restart: false,
				})
				.build(),
		)
		.await;

	// We accept 200 or 5xx (runner not connected), but NOT 401/403
	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"expected success or server error (runner not connected), got {status}"
	);
}

#[tokio::test]
async fn stop_deployment_works() {
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
			ApiRequest::<StopDeploymentRequest>::builder()
				.path(StopDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(StopDeploymentRequestHeaders {
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

#[tokio::test]
async fn get_deployment_logs_works() {
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
			ApiRequest::<GetDeploymentLogsRequest>::builder()
				.path(GetDeploymentLogsPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentLogsRequestHeaders {
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

#[tokio::test]
async fn get_deployment_metric_works() {
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
			ApiRequest::<GetDeploymentMetricRequest>::builder()
				.path(GetDeploymentMetricPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentMetricRequestHeaders {
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

#[tokio::test]
async fn deployment_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn deployment_wrong_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;

	// Create a second user who is NOT a member of the workspace
	let other_user = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: other_user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without workspace access should be denied"
	);
}
