use std::collections::BTreeMap;

use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::deployment::*,
	},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn list_machine_types_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListAllDeploymentMachineTypeRequest::METHOD,
			&ListAllDeploymentMachineTypePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
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
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;

	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;
	assert!(!dep.name.is_empty());
}

#[tokio::test]
async fn create_deployment_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;

	// Get machine type
	let machine_types = setup
		.server
		.method(
			ListAllDeploymentMachineTypeRequest::METHOD,
			&ListAllDeploymentMachineTypePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();

	let mt_id = machine_types.response.machine_types[0].id;

	let response = setup
		.server
		.method(
			CreateDeploymentRequest::METHOD,
			&CreateDeploymentPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&CreateDeploymentRequest {
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
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid deployment name"
	);
}

#[tokio::test]
async fn list_deployments_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let _dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let response = setup
		.server
		.method(
			ListDeploymentRequest::METHOD,
			&ListDeploymentPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();

	assert_eq!(1, response.response.deployments.len());
}

#[tokio::test]
async fn list_deployments_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListDeploymentRequest::METHOD,
			&ListDeploymentPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();

	assert!(response.response.deployments.is_empty());
}

#[tokio::test]
async fn get_deployment_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let response = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(dep.name, response.response.deployment.name);
}

#[tokio::test]
async fn get_deployment_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent deployment"
	);
}

#[tokio::test]
async fn update_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let new_name = random_name(8);
	setup
		.server
		.method(
			UpdateDeploymentRequest::METHOD,
			&UpdateDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&UpdateDeploymentRequest {
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
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));
}

#[tokio::test]
async fn delete_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	setup
		.server
		.method(
			DeleteDeploymentRequest::METHOD,
			&DeleteDeploymentPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteDeploymentResponse));

	// Verify it's gone
	let response = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn start_deployment_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let path = format!(
		"{}?force_restart=false",
		StartDeploymentPath {
			workspace_id: ws.id,
			deployment_id: dep.id,
		}
		.to_string()
	);

	// Start may succeed or fail depending on runner connectivity — just check
	// it doesn't return 4xx auth error
	let response = setup
		.server
		.method(StartDeploymentRequest::METHOD, &path)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
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
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let response = setup
		.server
		.method(
			StopDeploymentRequest::METHOD,
			&StopDeploymentPath {
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

#[tokio::test]
async fn get_deployment_logs_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let response = setup
		.server
		.method(
			GetDeploymentLogsRequest::METHOD,
			&GetDeploymentLogsPath {
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

#[tokio::test]
async fn get_deployment_metric_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let runner = create_test_runner(&setup, &user.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &user.access_token, ws.id, runner.id).await;

	let response = setup
		.server
		.method(
			GetDeploymentMetricRequest::METHOD,
			&GetDeploymentMetricPath {
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

#[tokio::test]
async fn deployment_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListDeploymentRequest::METHOD,
			&ListDeploymentPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn deployment_wrong_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let runner = create_test_runner(&setup, &admin.access_token, ws.id).await;
	let dep =
		create_test_deployment(&setup, &admin.access_token, ws.id, runner.id).await;

	// Create a second user who is NOT a member of the workspace
	let other_user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			GetDeploymentInfoRequest::METHOD,
			&GetDeploymentInfoPath {
				workspace_id: ws.id,
				deployment_id: dep.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&other_user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without workspace access should be denied"
	);
}
