use std::{collections::BTreeMap, str::FromStr};

use models::{
	ApiRequest,
	ApiSuccessResponseBody,
	api::{
		auth::*,
		workspace::deployment::*,
	},
	utils::BearerToken,
};

use crate::prelude::*;

/// Sign up and login, returning the access token.
async fn create_user_and_login(setup: &TestSetup) -> BearerToken {
	setup
		.make_api_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					username: "testuser".to_string(),
					password: "TestPassword123!".to_string(),
					first_name: "Test".to_string(),
					last_name: "User".to_string(),
					recovery_method: RecoveryMethod::Email {
						recovery_email: "test@example.com".to_string(),
					},
					cf_turnstile_token: "dummy-token".to_string(),
				})
				.build(),
		)
		.await
		.assert_status_success();

	let response = setup
		.make_api_call(
			ApiRequest::<LoginRequest>::builder()
				.headers(LoginRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(LoginRequest {
					user_id: "testuser".to_string(),
					password: "TestPassword123!".to_string(),
					mfa_otp: None,
					cf_turnstile_token: "dummy-token".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<LoginResponse>>()
		.response;

	BearerToken::from_str(&response.access_token).unwrap()
}

/// Create a deployment via HTTP and return its ID.
async fn create_deployment_via_http(
	setup: &TestSetup,
	token: &BearerToken,
	deploy_on_create: bool,
) -> Uuid {
	let response = setup
		.make_api_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: Uuid::nil(),
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: format!("dep-{}", &Uuid::new_v4().to_string()[..8]),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: Uuid::nil(),
					machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
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
					deploy_on_create,
				})
				.build(),
		)
		.await;
	response.assert_status(http::StatusCode::CREATED);

	response
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response
		.id
		.id
}

#[tokio::test]
async fn list_machine_types() {
	let setup = setup().await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath {
					workspace_id: Uuid::nil(),
				})
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	response.assert_status_success();

	let body = response
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>()
		.response;
	assert!(!body.machine_types.is_empty(), "expected machine types");
}

#[tokio::test]
async fn sign_up_and_login() {
	let setup = setup().await;
	let _token = create_user_and_login(&setup).await;
}

#[tokio::test]
async fn create_and_list_deployments() {
	let setup = setup().await;
	let token = create_user_and_login(&setup).await;
	let _dep_id = create_deployment_via_http(&setup, &token, true).await;

	setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: Uuid::nil(),
				})
				.query(ListResourceQuery {
					sort: Default::default(),
					search: Default::default(),
					count: 10,
					page: 0,
					additional_query: (),
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: token,
					user_agent: TEST_USER_AGENT,
				})
				.body(ListDeploymentRequest)
				.build(),
		)
		.await
		.assert_status_success();
}

#[tokio::test]
async fn create_and_delete_deployment() {
	let setup = setup().await;
	let token = create_user_and_login(&setup).await;
	let dep_id = create_deployment_via_http(&setup, &token, true).await;

	setup
		.make_api_call(
			ApiRequest::<DeleteDeploymentRequest>::builder()
				.path(DeleteDeploymentPath {
					workspace_id: Uuid::nil(),
					deployment_id: dep_id,
				})
				.headers(DeleteDeploymentRequestHeaders {
					authorization: token,
					user_agent: TEST_USER_AGENT,
				})
				.body(DeleteDeploymentRequest)
				.build(),
		)
		.await
		.assert_status(http::StatusCode::RESET_CONTENT);
}

#[tokio::test]
async fn start_stopped_deployment() {
	let setup = setup().await;
	let token = create_user_and_login(&setup).await;
	let dep_id = create_deployment_via_http(&setup, &token, false).await;

	// Deployment was created with deploy_on_create=false → status is Stopped.
	// Start it.
	setup
		.make_api_call(
			ApiRequest::<StartDeploymentRequest>::builder()
				.path(StartDeploymentPath {
					workspace_id: Uuid::nil(),
					deployment_id: dep_id,
				})
				.headers(StartDeploymentRequestHeaders {
					authorization: token,
					user_agent: TEST_USER_AGENT,
				})
				.body(StartDeploymentRequest)
				.build(),
		)
		.await
		.assert_status_success();
}

#[tokio::test]
async fn stop_running_deployment() {
	let setup = setup().await;
	let token = create_user_and_login(&setup).await;
	let dep_id = create_deployment_via_http(&setup, &token, true).await;

	setup
		.make_api_call(
			ApiRequest::<StopDeploymentRequest>::builder()
				.path(StopDeploymentPath {
					workspace_id: Uuid::nil(),
					deployment_id: dep_id,
				})
				.headers(StopDeploymentRequestHeaders {
					authorization: token,
					user_agent: TEST_USER_AGENT,
				})
				.body(StopDeploymentRequest)
				.build(),
		)
		.await
		.assert_status_success();
}

#[tokio::test]
async fn get_deployment_info() {
	let setup = setup().await;
	let token = create_user_and_login(&setup).await;
	let dep_id = create_deployment_via_http(&setup, &token, true).await;

	let body = setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: Uuid::nil(),
					deployment_id: dep_id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token,
					user_agent: TEST_USER_AGENT,
				})
				.body(GetDeploymentInfoRequest)
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>()
		.response;

	assert_eq!(body.deployment.id, dep_id);
}

#[tokio::test]
async fn unauthenticated_request_fails() {
	let setup = setup().await;

	// No authorization header → should fail with a client error.
	let response = setup
		.http
		.get("/workspace/00000000000000000000000000000000/deployment")
		.add_header(
			http::header::USER_AGENT,
			http::HeaderValue::from_static("cargo-test"),
		)
		.add_query_param("count", "10")
		.add_query_param("page", "0")
		.await;
	assert!(
		response.status_code().is_client_error(),
		"expected auth error, got {}",
		response.status_code()
	);
}
