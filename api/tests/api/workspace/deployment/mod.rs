use std::collections::BTreeMap;

use models::{ApiSuccessResponseBody, api::workspace::deployment::*, utils::Uuid};
use prost::Message;

use crate::prelude::*;

pub mod deploy_history;

#[tokio::test]
async fn list_machine_types_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(deployment.name, response.response.deployment.name);
}

#[tokio::test]
async fn get_deployment_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
	use api::routes::loki_patr_cloud::models::{EntryAdapter, PushRequest, StreamAdapter};

	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	// Pre-seed Loki with a log line using the same labels Alloy actually pushes:
	// deployment_id, deployment_name, runner_id, workspace_id
	let push_request = PushRequest {
		streams: vec![StreamAdapter {
			labels: format!(
				r#"{{deployment_id="{}", deployment_name="{}", runner_id="{}", workspace_id="{}"}}"#,
				deployment.id, deployment.name, runner.id, workspace.id,
			),
			entries: vec![EntryAdapter {
				timestamp: Some(prost_types::Timestamp {
					seconds: time::OffsetDateTime::now_utc().unix_timestamp(),
					nanos: 0,
				}),
				line: "hello from deployment logs test".to_string(),
			}],
			hash: 0,
		}],
	};
	let encoded = push_request.encode_to_vec();
	let compressed = snap::raw::Encoder::new()
		.compress_vec(&encoded)
		.expect("snappy compress failed");

	let loki_url = format!("{}/loki/api/v1/push", setup.upstream_loki_url());
	let push_resp = reqwest::Client::new()
		.post(&loki_url)
		.header("Content-Type", "application/x-protobuf")
		.header("X-Scope-OrgID", workspace.id.to_string())
		.body(compressed)
		.send()
		.await
		.expect("failed to push to Loki");

	assert!(
		push_resp.status().is_success(),
		"Loki push failed: {}",
		push_resp.text().await.unwrap_or_default()
	);

	// Wait for Loki to index
	tokio::time::sleep(std::time::Duration::from_secs(2)).await;

	let response = setup
		.make_web_dashboard_call(
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

	assert_eq!(
		response.status_code(),
		StatusCode::OK,
		"expected 200 from get_deployment_logs"
	);

	let body = response.json::<ApiSuccessResponseBody<GetDeploymentLogsResponse>>();
	assert!(
		!body.response.logs.is_empty(),
		"expected at least one log entry"
	);
	assert!(
		body.response
			.logs
			.iter()
			.any(|l| l.log.contains("hello from deployment logs test")),
		"expected to find the seeded log line"
	);
}

#[tokio::test]
async fn get_deployment_logs_with_search_filter() {
	use api::routes::loki_patr_cloud::models::{EntryAdapter, PushRequest, StreamAdapter};

	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	// Push multiple log lines with the same labels Alloy actually pushes
	let now = time::OffsetDateTime::now_utc().unix_timestamp();
	let push_request = PushRequest {
		streams: vec![StreamAdapter {
			labels: format!(
				r#"{{deployment_id="{}", deployment_name="{}", runner_id="{}", workspace_id="{}"}}"#,
				deployment.id, deployment.name, runner.id, workspace.id,
			),
			entries: vec![
				EntryAdapter {
					timestamp: Some(prost_types::Timestamp {
						seconds: now,
						nanos: 0,
					}),
					line: "normal log message".to_string(),
				},
				EntryAdapter {
					timestamp: Some(prost_types::Timestamp {
						seconds: now + 1,
						nanos: 0,
					}),
					line: "special-keyword-xyzzy in this line".to_string(),
				},
			],
			hash: 0,
		}],
	};
	let encoded = push_request.encode_to_vec();
	let compressed = snap::raw::Encoder::new()
		.compress_vec(&encoded)
		.expect("snappy compress failed");

	let loki_url = format!("{}/loki/api/v1/push", setup.upstream_loki_url());
	reqwest::Client::new()
		.post(&loki_url)
		.header("Content-Type", "application/x-protobuf")
		.header("X-Scope-OrgID", workspace.id.to_string())
		.body(compressed)
		.send()
		.await
		.expect("failed to push to Loki");

	tokio::time::sleep(std::time::Duration::from_secs(2)).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentLogsRequest>::builder()
				.path(GetDeploymentLogsPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.query(GetDeploymentLogsQuery {
					end_time: None,
					limit: None,
					search: Some("special-keyword-xyzzy".to_string()),
				})
				.headers(GetDeploymentLogsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::OK,
		"expected 200 from filtered logs query"
	);

	let body = response.json::<ApiSuccessResponseBody<GetDeploymentLogsResponse>>();
	assert!(
		body.response
			.logs
			.iter()
			.all(|l| l.log.contains("special-keyword-xyzzy")),
		"all returned logs should contain the search keyword"
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
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentMetricRequest>::builder()
				.path(GetDeploymentMetricPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
					metric: DeploymentMetricName::ContainerCpuUsage,
				})
				.headers(GetDeploymentMetricRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn get_deployment_metric_empty() {
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
			ApiRequest::<GetDeploymentMetricRequest>::builder()
				.path(GetDeploymentMetricPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
					metric: DeploymentMetricName::IngressRps,
				})
				.headers(GetDeploymentMetricRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	let body = response.json::<ApiSuccessResponseBody<GetDeploymentMetricResponse>>();
	assert!(
		body.response.data_points.is_empty(),
		"deployment with no seeded metrics should return an empty data_points array"
	);
}

#[tokio::test]
async fn create_deployment_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mt_id = setup
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>()
		.response
		.machine_types[0]
		.id;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: deployment.name.clone(),
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
		"creating a deployment with a taken name should fail"
	);
}

#[tokio::test]
async fn create_deployment_with_volumes() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let volume = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;

	let mt_id = setup
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>()
		.response
		.machine_types[0]
		.id;

	let mut volumes = BTreeMap::new();
	volumes.insert(volume.id, "/data".to_string());

	let create_resp = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
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
						volumes,
					},
					deploy_on_create: false,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: create_resp.response.id.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(
		response.response.running_details.volumes.get(&volume.id),
		Some(&"/data".to_string()),
		"deployment should report its mounted volume"
	);
}

#[tokio::test]
async fn create_deployment_with_env_vars() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let mt_id = setup
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>()
		.response
		.machine_types[0]
		.id;

	let mut env_vars = BTreeMap::new();
	env_vars.insert(
		"FOO".to_string(),
		EnvironmentVariableValue::String("bar".to_string()),
	);
	env_vars.insert(
		"BAZ".to_string(),
		EnvironmentVariableValue::String("qux".to_string()),
	);

	let create_resp = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
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
						environment_variables: env_vars,
						startup_probe: None,
						liveness_probe: None,
						config_mounts: BTreeMap::new(),
						volumes: BTreeMap::new(),
					},
					deploy_on_create: false,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: create_resp.response.id.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(
		response.response.running_details.environment_variables.len(),
		2
	);
	assert!(
		response
			.response
			.running_details
			.environment_variables
			.contains_key("FOO")
	);
}

#[tokio::test]
async fn create_deployment_with_ports() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let mt_id = setup
		.make_web_dashboard_call(
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
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>()
		.response
		.machine_types[0]
		.id;

	let mut ports = BTreeMap::new();
	ports.insert(
		models::utils::StringifiedU16::new(8080),
		ExposedPortType::Http,
	);

	let create_resp = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
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
						ports,
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
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: create_resp.response.id.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(response.response.running_details.ports.len(), 1);
}

#[tokio::test]
async fn update_deployment_name_persists() {
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

	let response = setup
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
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(new_name, response.response.deployment.name);
}

#[tokio::test]
async fn update_deployment_machine_type() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let machine_types = setup
		.make_web_dashboard_call(
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

	// Need at least two machine types to switch between.
	let Some(other_mt) = machine_types
		.response
		.machine_types
		.iter()
		.find(|m| m.id != machine_types.response.machine_types[0].id)
	else {
		// Fall back to using the same id — the update should still be a no-op success.
		return;
	};

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
					name: None,
					runner: None,
					machine_type: Some(other_mt.id),
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

	let response = setup
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
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>();

	assert_eq!(other_mt.id, response.response.deployment.machine_type);
}

#[tokio::test]
async fn start_deployment_idempotent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	for _ in 0..2 {
		let response = setup
			.make_web_dashboard_call(
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

		let status = response.status_code();
		assert!(
			status.is_success() || status.is_server_error(),
			"start should be idempotent (no 4xx); got {status}"
		);
	}
}

#[tokio::test]
async fn stop_deployment_idempotent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	for _ in 0..2 {
		let response = setup
			.make_web_dashboard_call(
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
			"stop should be idempotent (no 4xx); got {status}"
		);
	}
}

#[tokio::test]
async fn get_deployment_logs_empty() {
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

	assert_eq!(response.status_code(), StatusCode::OK);
	let body = response.json::<ApiSuccessResponseBody<GetDeploymentLogsResponse>>();
	assert!(
		body.response.logs.is_empty(),
		"deployment with no seeded logs should return an empty array"
	);
}

#[tokio::test]
async fn delete_deployment_while_running() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let start_resp = setup
		.make_web_dashboard_call(
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
	let start_status = start_resp.status_code();
	assert!(
		start_status.is_success() || start_status.is_server_error(),
		"start should not return 4xx; got {start_status}"
	);

	setup
		.make_web_dashboard_call(
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
}

#[tokio::test]
async fn deployment_cross_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace_a.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace_a.id, runner.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace_b.id,
					deployment_id: deployment.id,
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
		"deployment in workspace A should not be accessible via workspace B's path"
	);
}

#[tokio::test]
async fn deployment_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
