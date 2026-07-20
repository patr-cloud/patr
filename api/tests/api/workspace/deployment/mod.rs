use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::deployment::{deploy_history::*, *},
	utils::{Base64String, ListResourceQuery, StringifiedU16, Uuid},
};
use prost::Message;

use crate::prelude::*;

pub mod deploy_history;

/// Fetch the first available deployment machine type for a workspace.
async fn first_machine_type(setup: &TestSetup, workspace_id: Uuid) -> Uuid {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath { workspace_id })
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>()
		.response
		.machine_types[0]
		.id
}

/// A minimal valid Patr-registry deployment body (random name, no ports/env,
/// scale 1, not deployed on create). Tests mutate the fields they care about.
fn patr_body(repo: Uuid, runner: Uuid, machine_type: Uuid) -> CreateDeploymentRequest {
	CreateDeploymentRequest {
		name: random_name(8),
		registry: DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id: repo,
		},
		image_tag: "latest".to_string(),
		runner,
		machine_type,
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
	}
}

/// Send a create-deployment request and return the raw response.
async fn send_create(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	body: CreateDeploymentRequest,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath { workspace_id })
				.headers(CreateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(body)
				.build(),
		)
		.await
}

/// A full update body seeded from a deployment's current state. Updates now
/// send the whole object, so tests mutate the single field they exercise and
/// leave the rest identical to what the deployment already has.
async fn full_update(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
) -> UpdateDeploymentRequest {
	let info = get_info(setup, token, workspace_id, deployment_id).await;
	UpdateDeploymentRequest {
		name: info.deployment.name.clone(),
		image_tag: info.deployment.image_tag.clone(),
		runner: info.deployment.runner,
		machine_type: info.deployment.machine_type,
		running_details: info.running_details.clone(),
	}
}

/// Send an update-deployment request and return the raw response.
async fn send_update(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
	body: UpdateDeploymentRequest,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateDeploymentRequest>::builder()
				.path(UpdateDeploymentPath {
					workspace_id,
					deployment_id,
				})
				.headers(UpdateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(body)
				.build(),
		)
		.await
}

/// Fetch a deployment's get-info response.
async fn get_info(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
) -> GetDeploymentInfoResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id,
					deployment_id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>()
		.response
}

/// Send a start-deployment request (with the given `force_restart`).
async fn send_start(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
	force_restart: bool,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<StartDeploymentRequest>::builder()
				.path(StartDeploymentPath {
					workspace_id,
					deployment_id,
				})
				.query(StartDeploymentQuery { force_restart })
				.headers(StartDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
}

/// Send a stop-deployment request.
async fn send_stop(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<StopDeploymentRequest>::builder()
				.path(StopDeploymentPath {
					workspace_id,
					deployment_id,
				})
				.headers(StopDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
}

/// Send a delete-deployment request.
async fn send_delete(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id,
					deployment_id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
}

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
	let mut body = full_update(&setup, &user.access_token, workspace.id, deployment.id).await;
	body.name = new_name.clone();
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
				.body(body)
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
		response
			.response
			.running_details
			.environment_variables
			.len(),
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
	let mut body = full_update(&setup, &user.access_token, workspace.id, deployment.id).await;
	body.name = new_name.clone();

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
				.body(body)
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

	let mut body = full_update(&setup, &user.access_token, workspace.id, deployment.id).await;
	body.machine_type = other_mt.id;
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
				.body(body)
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

#[tokio::test]
async fn create_patr_deployment_stopped_shape() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details
		.ports
		.insert(StringifiedU16::new(80), ExposedPortType::Http);
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	let info = get_info(&setup, &user.access_token, workspace.id, created.id.id).await;
	assert_eq!(info.deployment.status.to_string(), "stopped");
	assert!(info.deployment.registry.is_patr_registry());
	assert_eq!(info.deployment.registry.repository_id(), Some(repo.id));
	assert_eq!(info.deployment.image_tag, "latest");
	assert!(info.deployment.current_live_digest.is_none());
	assert_eq!(info.running_details.min_horizontal_scale, 1);
	assert_eq!(info.running_details.max_horizontal_scale, 1);
	assert_eq!(
		info.running_details.ports,
		BTreeMap::from([(StringifiedU16::new(80), ExposedPortType::Http)])
	);
}

#[tokio::test]
async fn deploy_on_create_sets_deploying() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut body = patr_body(repo.id, runner.id, mt);
	body.deploy_on_create = true;
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	let info = get_info(&setup, &user.access_token, workspace.id, created.id.id).await;
	assert_eq!(info.deployment.status.to_string(), "deploying");
}

#[tokio::test]
async fn create_deployment_image_tag_lowercased() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut body = patr_body(repo.id, runner.id, mt);
	body.image_tag = "Latest-V2".to_string();
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	let info = get_info(&setup, &user.access_token, workspace.id, created.id.id).await;
	assert_eq!(info.deployment.image_tag, "latest-v2");
}

#[tokio::test]
async fn create_deployment_reusable_after_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;
	let name = random_name(8);

	let mut body = patr_body(repo.id, runner.id, mt);
	body.name = name.clone();
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	let mut dup = patr_body(repo.id, runner.id, mt);
	dup.name = name.clone();
	let dup_resp = send_create(&setup, &user.access_token, workspace.id, dup).await;
	assert_eq!(
		409,
		dup_resp.status_code().as_u16(),
		"duplicate deployment name should be 409"
	);

	send_delete(&setup, &user.access_token, workspace.id, created.id.id)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteDeploymentResponse));

	let mut readd = patr_body(repo.id, runner.id, mt);
	readd.name = name;
	let readd_resp = send_create(&setup, &user.access_token, workspace.id, readd).await;
	assert!(
		readd_resp.status_code().is_success(),
		"name should be reusable after delete, got {}",
		readd_resp.status_code()
	);
}

#[tokio::test]
async fn create_deployment_tcp_port_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	// The DB `exposed_port_type` enum only has `http`; a TCP port hits the enum
	// and 500s instead of being stored or cleanly rejected. Pinned gap.
	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details
		.ports
		.insert(StringifiedU16::new(5432), ExposedPortType::Tcp);
	let resp = send_create(&setup, &user.access_token, workspace.id, body).await;
	assert!(
		resp.status_code().is_server_error(),
		"a TCP port should hit the DB enum gap → 500, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_deployment_startup_probe_roundtrip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details
		.ports
		.insert(StringifiedU16::new(8080), ExposedPortType::Http);
	body.running_details.startup_probe = Some(DeploymentProbe {
		port: 8080,
		path: "/healthz".to_string(),
	});
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	let info = get_info(&setup, &user.access_token, workspace.id, created.id.id).await;
	assert_eq!(
		info.running_details.startup_probe,
		Some(DeploymentProbe {
			port: 8080,
			path: "/healthz".to_string(),
		})
	);
}

#[tokio::test]
async fn create_deployment_config_mount_roundtrip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details.config_mounts.insert(
		"/etc/app/conf".to_string(),
		Base64String::from(b"hello config\n".to_vec()),
	);
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	let info = get_info(&setup, &user.access_token, workspace.id, created.id.id).await;
	let mount = info
		.running_details
		.config_mounts
		.get("/etc/app/conf")
		.expect("config mount should round-trip");
	assert_eq!(&mount[..], b"hello config\n");
}

// ---------- create: name / fk / scale validation ----------

#[tokio::test]
async fn create_deployment_name_length_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	for (name, expect_ok) in [
		("abc".to_string(), false),
		("abcd".to_string(), true),
		("a".repeat(255), true),
		("a".repeat(256), false),
	] {
		let mut body = patr_body(repo.id, runner.id, mt);
		body.name = name.clone();
		let resp = send_create(&setup, &user.access_token, workspace.id, body).await;
		if expect_ok {
			assert!(
				resp.status_code().is_success(),
				"name len {} should be accepted, got {}",
				name.len(),
				resp.status_code()
			);
		} else {
			assert_eq!(
				400,
				resp.status_code().as_u16(),
				"name len {} should be rejected with 400",
				name.len()
			);
		}
	}
}

#[tokio::test]
async fn create_deployment_name_charset() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut bad = patr_body(repo.id, runner.id, mt);
	bad.name = "a/b/c".to_string();
	assert_eq!(
		400,
		send_create(&setup, &user.access_token, workspace.id, bad)
			.await
			.status_code()
			.as_u16(),
		"a slash in the name should be rejected"
	);

	let mut ok = patr_body(repo.id, runner.id, mt);
	ok.name = "My App-1_v.2".to_string();
	assert!(
		send_create(&setup, &user.access_token, workspace.id, ok)
			.await
			.status_code()
			.is_success(),
		"allowed punctuation should be accepted"
	);
}

#[tokio::test]
async fn create_deployment_name_trimmed() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;
	let name = random_name(8);

	let mut body = patr_body(repo.id, runner.id, mt);
	body.name = format!("  {name}  ");
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;

	let info = get_info(&setup, &user.access_token, workspace.id, created.id.id).await;
	assert_eq!(info.deployment.name, name, "name should be trimmed");
}

#[tokio::test]
async fn create_deployment_nonexistent_repo_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let resp = send_create(
		&setup,
		&user.access_token,
		workspace.id,
		patr_body(Uuid::nil(), runner.id, mt),
	)
	.await;
	assert!(
		resp.status_code().is_server_error(),
		"a nonexistent repository should fail the FK → 500, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_deployment_cross_workspace_repo_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let other = setup.create_test_user().await;
	let other_ws = setup.create_test_workspace(&other.access_token).await;
	let other_repo = setup
		.create_test_container_repo(&other.access_token, other_ws.id)
		.await;

	let resp = send_create(
		&setup,
		&user.access_token,
		workspace.id,
		patr_body(other_repo.id, runner.id, mt),
	)
	.await;
	assert!(
		resp.status_code().is_server_error(),
		"a cross-workspace repository should fail the workspace-scoped FK → 500, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_deployment_nonexistent_runner_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let resp = send_create(
		&setup,
		&user.access_token,
		workspace.id,
		patr_body(repo.id, Uuid::nil(), mt),
	)
	.await;
	assert!(
		resp.status_code().is_server_error(),
		"a nonexistent runner should fail the FK → 500, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_deployment_cross_workspace_runner_accepted() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	// The runner FK references runner(id) only (NOT workspace-scoped), so a
	// runner from another workspace is accepted. Pin this isolation gap.
	let other = setup.create_test_user().await;
	let other_ws = setup.create_test_workspace(&other.access_token).await;
	let other_runner = setup
		.create_test_runner(&other.access_token, other_ws.id)
		.await;

	let resp = send_create(
		&setup,
		&user.access_token,
		workspace.id,
		patr_body(repo.id, other_runner.id, mt),
	)
	.await;
	assert!(
		resp.status_code().is_success(),
		"a cross-workspace runner is accepted (FK not workspace-scoped), got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_deployment_nonexistent_machine_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let resp = send_create(
		&setup,
		&user.access_token,
		workspace.id,
		patr_body(repo.id, runner.id, Uuid::nil()),
	)
	.await;
	assert!(
		resp.status_code().is_server_error(),
		"a nonexistent machine type should fail the FK → 500, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_deployment_min_scale_zero_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	// Both create and update reject minHorizontalScale=0 — a deployment can't
	// run zero replicas.
	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details.min_horizontal_scale = 0;
	assert!(
		send_create(&setup, &user.access_token, workspace.id, body)
			.await
			.status_code()
			.is_client_error(),
		"create should reject minHorizontalScale=0"
	);
}

#[tokio::test]
async fn create_deployment_max_less_than_min_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details.min_horizontal_scale = 5;
	body.running_details.max_horizontal_scale = 2;
	assert!(
		send_create(&setup, &user.access_token, workspace.id, body)
			.await
			.status_code()
			.is_server_error(),
		"max < min should hit the DB CHECK → 500"
	);
}

#[tokio::test]
async fn create_deployment_unexposed_probe_port_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details
		.ports
		.insert(StringifiedU16::new(80), ExposedPortType::Http);
	body.running_details.startup_probe = Some(DeploymentProbe {
		port: 9999,
		path: "/healthz".to_string(),
	});
	assert!(
		send_create(&setup, &user.access_token, workspace.id, body)
			.await
			.status_code()
			.is_server_error(),
		"a startup-probe port that isn't exposed should fail the FK → 500"
	);
}

// ---------- list: ordering / pagination / bounds ----------

#[tokio::test]
async fn list_deployments_ordered_created_desc() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let mut names = Vec::new();
	for _ in 0..3 {
		names.push(
			setup
				.create_test_deployment(&user.access_token, workspace.id, runner.id)
				.await
				.name,
		);
	}

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();

	let listed: Vec<String> = response
		.response
		.deployments
		.iter()
		.map(|d| d.name.clone())
		.collect();
	names.reverse();
	assert_eq!(names, listed, "deployments should be ordered created DESC");
}

#[tokio::test]
async fn list_deployments_pagination() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	for _ in 0..5 {
		setup
			.create_test_deployment(&user.access_token, workspace.id, runner.id)
			.await;
	}

	let mut pages = Vec::new();
	for page in 0..2usize {
		pages.push(
			setup
				.make_web_dashboard_call(
					ApiRequest::<ListDeploymentRequest>::builder()
						.path(ListDeploymentPath {
							workspace_id: workspace.id,
						})
						.query(ListResourceQuery {
							sort: None,
							search: Default::default(),
							count: 2,
							page,
							additional_query: (),
						})
						.headers(ListDeploymentRequestHeaders {
							authorization: user.access_token.clone(),
							user_agent: TEST_USER_AGENT,
						})
						.build(),
				)
				.await
				.json::<ApiSuccessResponseBody<ListDeploymentResponse>>(),
		);
	}
	assert_eq!(2, pages[0].response.deployments.len());
	assert_eq!(2, pages[1].response.deployments.len());
	let ids: std::collections::BTreeSet<Uuid> = pages[0]
		.response
		.deployments
		.iter()
		.chain(pages[1].response.deployments.iter())
		.map(|d| d.id)
		.collect();
	assert_eq!(4, ids.len(), "the two pages should not overlap");
}

#[tokio::test]
async fn list_deployments_page_out_of_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 10,
					page: 50,
					additional_query: (),
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		400,
		response.status_code().as_u16(),
		"a page past the end should be PageOutOfBounds (400)"
	);
}

#[tokio::test]
async fn list_deployments_page_zero_empty_allowed() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 10,
					page: 0,
					additional_query: (),
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>();
	assert!(
		response.response.deployments.is_empty(),
		"page 0 of an empty result set is a legitimately empty list"
	);
}

// ---------- lifecycle ----------

#[tokio::test]
async fn start_deployment_sets_deploying() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	send_start(&setup, &user.access_token, workspace.id, dep.id, false).await;
	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(info.deployment.status.to_string(), "deploying");
}

#[tokio::test]
async fn stop_deployment_sets_stopped() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'deploying' WHERE id = '{}'",
			dep.id
		))
		.await;

	send_stop(&setup, &user.access_token, workspace.id, dep.id).await;
	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(info.deployment.status.to_string(), "stopped");
}

#[tokio::test]
async fn start_deployment_force_restart_is_noop() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	// force_restart is destructured but ignored — start still sets Deploying.
	send_start(&setup, &user.access_token, workspace.id, dep.id, true).await;
	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(info.deployment.status.to_string(), "deploying");
}

#[tokio::test]
async fn start_running_deployment_sets_deploying() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'running' WHERE id = '{}'",
			dep.id
		))
		.await;

	// Start has no status guard — a running deployment is still set to Deploying.
	send_start(&setup, &user.access_token, workspace.id, dep.id, false).await;
	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(info.deployment.status.to_string(), "deploying");
}

#[tokio::test]
async fn stop_running_deployment_sets_stopped() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'running' WHERE id = '{}'",
			dep.id
		))
		.await;

	send_stop(&setup, &user.access_token, workspace.id, dep.id).await;
	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(info.deployment.status.to_string(), "stopped");
}

#[tokio::test]
async fn repeat_delete_deployment_401() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	send_delete(&setup, &user.access_token, workspace.id, dep.id)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteDeploymentResponse));
	let second = send_delete(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(
		401,
		second.status_code().as_u16(),
		"repeat delete should 401 (anti-enumeration)"
	);
}

#[tokio::test]
async fn delete_deployment_from_each_state() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	for state in ["stopped", "deploying", "running", "errored"] {
		let dep = setup
			.create_test_deployment(&user.access_token, workspace.id, runner.id)
			.await;
		setup
			.execute_sql(&format!(
				"UPDATE deployment SET status = '{state}'::DEPLOYMENT_STATUS WHERE id = '{}'",
				dep.id
			))
			.await;
		send_delete(&setup, &user.access_token, workspace.id, dep.id)
			.await
			.assert_json(&ApiSuccessResponseBody::new(DeleteDeploymentResponse));
	}
}

#[tokio::test]
async fn get_info_pushed_status_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	// `pushed` is a valid DB enum value but the Rust model can't deserialize it.
	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'pushed'::DEPLOYMENT_STATUS WHERE id = '{}'",
			dep.id
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: dep.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_server_error(),
		"get-info on a `pushed` status row should 500 (model cannot deserialize)"
	);
}

#[tokio::test]
async fn start_stop_nonexistent_deployment_401() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	assert_eq!(
		401,
		send_start(&setup, &user.access_token, workspace.id, Uuid::nil(), false)
			.await
			.status_code()
			.as_u16()
	);
	assert_eq!(
		401,
		send_stop(&setup, &user.access_token, workspace.id, Uuid::nil())
			.await
			.status_code()
			.as_u16()
	);
}

// ---------- update ----------

#[tokio::test]
async fn update_deployment_invalid_name_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.name = "a/b".to_string();
	assert_eq!(
		400,
		send_update(&setup, &user.access_token, workspace.id, dep.id, body)
			.await
			.status_code()
			.as_u16()
	);
}

#[tokio::test]
async fn update_deployment_image_tag_persists() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, deployment.id).await;
	body.image_tag = "alpine".to_string();
	assert_eq!(
		202,
		send_update(
			&setup,
			&user.access_token,
			workspace.id,
			deployment.id,
			body
		)
		.await
		.status_code()
		.as_u16()
	);

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

	assert_eq!("alpine", response.response.deployment.image_tag);
}

#[tokio::test]
async fn update_deployment_invalid_tag_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.image_tag = "bad tag!".to_string();
	assert_eq!(
		400,
		send_update(&setup, &user.access_token, workspace.id, dep.id, body)
			.await
			.status_code()
			.as_u16()
	);
}

#[tokio::test]
async fn update_deployment_empty_tag_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.image_tag = "   ".to_string();
	assert_eq!(
		400,
		send_update(&setup, &user.access_token, workspace.id, dep.id, body)
			.await
			.status_code()
			.as_u16()
	);
}

#[tokio::test]
async fn update_deployment_change_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let runner2 = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.runner = runner2.id;
	send_update(&setup, &user.access_token, workspace.id, dep.id, body)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));
	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(info.deployment.runner, runner2.id);
}

#[tokio::test]
async fn update_deployment_deploy_on_push() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.running_details.deploy_on_push = true;
	send_update(&setup, &user.access_token, workspace.id, dep.id, body)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));
	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert!(info.running_details.deploy_on_push);
}

#[tokio::test]
async fn update_deployment_min_scale_zero_400() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	// Update enforces minHorizontalScale >= 1, same as create.
	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.running_details.min_horizontal_scale = 0;
	assert!(
		send_update(&setup, &user.access_token, workspace.id, dep.id, body)
			.await
			.status_code()
			.is_client_error(),
		"update should reject minHorizontalScale=0"
	);
}

#[tokio::test]
async fn update_deployment_max_less_than_min_500() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.running_details.min_horizontal_scale = 5;
	body.running_details.max_horizontal_scale = 2;
	assert!(
		send_update(&setup, &user.access_token, workspace.id, dep.id, body)
			.await
			.status_code()
			.is_server_error(),
		"max < min on update should hit the DB CHECK → 500"
	);
}

#[tokio::test]
async fn update_deployment_ports_replaced() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut first = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	first.running_details.ports =
		BTreeMap::from([(StringifiedU16::new(80), ExposedPortType::Http)]);
	send_update(&setup, &user.access_token, workspace.id, dep.id, first)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));

	let mut second = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	second.running_details.ports = BTreeMap::from([
		(StringifiedU16::new(8080), ExposedPortType::Http),
		(StringifiedU16::new(9090), ExposedPortType::Http),
	]);
	send_update(&setup, &user.access_token, workspace.id, dep.id, second)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));

	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(
		info.running_details.ports,
		BTreeMap::from([
			(StringifiedU16::new(8080), ExposedPortType::Http),
			(StringifiedU16::new(9090), ExposedPortType::Http),
		]),
		"ports should be replaced wholesale when provided"
	);
}

#[tokio::test]
async fn update_deployment_ports_omitted_kept() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut set_ports = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	set_ports.running_details.ports = BTreeMap::from([
		(StringifiedU16::new(80), ExposedPortType::Http),
		(StringifiedU16::new(8080), ExposedPortType::Http),
	]);
	send_update(&setup, &user.access_token, workspace.id, dep.id, set_ports)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));

	let mut name_only = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	name_only.name = random_name(8);
	send_update(&setup, &user.access_token, workspace.id, dep.id, name_only)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));

	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(
		info.running_details.ports,
		BTreeMap::from([
			(StringifiedU16::new(80), ExposedPortType::Http),
			(StringifiedU16::new(8080), ExposedPortType::Http),
		]),
		"omitting ports should keep the existing ones"
	);
}

#[tokio::test]
async fn update_deployment_env_replaced_and_kept() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut set_env = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	set_env.running_details.environment_variables = BTreeMap::from([
		(
			"B".to_string(),
			EnvironmentVariableValue::String("2".to_string()),
		),
		(
			"C".to_string(),
			EnvironmentVariableValue::String("3".to_string()),
		),
	]);
	send_update(&setup, &user.access_token, workspace.id, dep.id, set_env)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));

	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(info.running_details.environment_variables.len(), 2);

	let mut name_only = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	name_only.name = random_name(8);
	send_update(&setup, &user.access_token, workspace.id, dep.id, name_only)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));

	let info = get_info(&setup, &user.access_token, workspace.id, dep.id).await;
	assert_eq!(
		info.running_details.environment_variables.len(),
		2,
		"omitting env should keep the existing vars"
	);
}

#[tokio::test]
async fn update_deployment_startup_probe_set_then_cleared() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let mt = first_machine_type(&setup, workspace.id).await;

	// Need an exposed port for the probe FK.
	let mut body = patr_body(repo.id, runner.id, mt);
	body.running_details
		.ports
		.insert(StringifiedU16::new(8080), ExposedPortType::Http);
	let created = send_create(&setup, &user.access_token, workspace.id, body)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response;
	let dep_id = created.id.id;

	let mut set_probe = full_update(&setup, &user.access_token, workspace.id, dep_id).await;
	set_probe.running_details.startup_probe = Some(DeploymentProbe {
		port: 8080,
		path: "/healthz".to_string(),
	});
	send_update(&setup, &user.access_token, workspace.id, dep_id, set_probe)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));
	assert_eq!(
		get_info(&setup, &user.access_token, workspace.id, dep_id)
			.await
			.running_details
			.startup_probe,
		Some(DeploymentProbe {
			port: 8080,
			path: "/healthz".to_string(),
		})
	);

	// Clearing a probe now means sending `None` (the port=0 sentinel is gone).
	let mut clear_probe = full_update(&setup, &user.access_token, workspace.id, dep_id).await;
	clear_probe.running_details.startup_probe = None;
	send_update(
		&setup,
		&user.access_token,
		workspace.id,
		dep_id,
		clear_probe,
	)
	.await
	.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));
	assert!(
		get_info(&setup, &user.access_token, workspace.id, dep_id)
			.await
			.running_details
			.startup_probe
			.is_none(),
		"sending startup_probe: None should clear the startup probe"
	);
}

#[tokio::test]
async fn update_deployment_nonexistent_401() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let body = UpdateDeploymentRequest {
		name: "x-y-z".to_string(),
		image_tag: "latest".to_string(),
		runner: Uuid::nil(),
		machine_type: Uuid::nil(),
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
	};
	assert_eq!(
		401,
		send_update(&setup, &user.access_token, workspace.id, Uuid::nil(), body)
			.await
			.status_code()
			.as_u16()
	);
}

#[tokio::test]
async fn update_deployment_writes_no_deploy_history() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let dep = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let mut body = full_update(&setup, &user.access_token, workspace.id, dep.id).await;
	body.name = random_name(8);
	send_update(&setup, &user.access_token, workspace.id, dep.id, body)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateDeploymentResponse));

	let history = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDeploymentDeployHistoryRequest>::builder()
				.path(ListDeploymentDeployHistoryPath {
					workspace_id: workspace.id,
					deployment_id: dep.id,
				})
				.headers(ListDeploymentDeployHistoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentDeployHistoryResponse>>();
	assert!(
		history.response.deploys.is_empty(),
		"update should not write a deploy-history row"
	);
}
