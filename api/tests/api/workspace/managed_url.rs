use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::{
		WithId,
		workspace::{deployment::*, managed_url::*},
	},
	utils::{ListResourceQuery, StringifiedU16},
};

use crate::prelude::*;

fn rand_subdomain() -> String {
	random_name(6)
}

/// Create a domain and mark it verified (the precondition for managed URLs).
async fn verified_domain(setup: &TestSetup, token: &BearerToken, ws: Uuid) -> Uuid {
	let domain = setup.create_test_domain(token, ws).await;
	setup.mark_test_domain_verified(domain.id).await;
	domain.id
}

/// Create an external deployment that exposes `port` (so the managed-url FK
/// `managed_url_fk_deployment_id_port` is satisfiable).
async fn deployment_with_port(
	setup: &TestSetup,
	token: &BearerToken,
	ws: Uuid,
	runner: Uuid,
	port: u16,
) -> Uuid {
	let machine_type = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath { workspace_id: ws })
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
	ports.insert(StringifiedU16::new(port), ExposedPortType::Http);

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath { workspace_id: ws })
				.headers(CreateDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner,
					machine_type,
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
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response
		.id
		.id
}

/// A workspace with a verified domain + a deployment exposing port 80.
struct MuFixture {
	user: TestUser,
	ws: Uuid,
	domain: Uuid,
	deployment: Uuid,
}

async fn mu_fixture(setup: &TestSetup) -> MuFixture {
	let user = setup.create_test_user().await;
	let ws = setup.create_test_workspace(&user.access_token).await.id;
	let domain = verified_domain(setup, &user.access_token, ws).await;
	let runner = setup.create_test_runner(&user.access_token, ws).await;
	let deployment = deployment_with_port(setup, &user.access_token, ws, runner.id, 80).await;
	MuFixture {
		user,
		ws,
		domain,
		deployment,
	}
}

fn proxy_body(
	domain: Uuid,
	deployment: Uuid,
	port: u16,
	sub: String,
	path: &str,
) -> CreateManagedURLRequest {
	CreateManagedURLRequest {
		sub_domain: sub,
		domain_id: domain,
		path: path.to_string(),
		url_type: ManagedUrlType::ProxyDeployment {
			deployment_id: deployment,
			port,
		},
	}
}

async fn send_create_mu(
	setup: &TestSetup,
	token: &BearerToken,
	ws: Uuid,
	body: CreateManagedURLRequest,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath { workspace_id: ws })
				.headers(CreateManagedURLRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(body)
				.build(),
		)
		.await
}

async fn list_mus(setup: &TestSetup, token: &BearerToken, ws: Uuid) -> Vec<WithId<ManagedUrl>> {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath { workspace_id: ws })
				.headers(ListManagedURLRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>()
		.response
		.urls
}

#[tokio::test]
async fn create_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;
	assert_ne!(url_id, Uuid::nil());
}

#[tokio::test]
async fn list_managed_urls_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let _url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	assert_eq!(1, response.response.urls.len());
}

#[tokio::test]
async fn list_managed_urls_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	assert!(response.response.urls.is_empty());
}

#[tokio::test]
async fn update_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: "/updated".to_string(),
					url_type: ManagedUrlType::Redirect {
						url: "https://example.com".to_string(),
						permanent_redirect: false,
						http_only: false,
					},
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateManagedURLResponse));
}

#[tokio::test]
async fn delete_managed_url_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteManagedURLResponse));
}

#[tokio::test]
async fn verify_configuration_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyManagedURLConfigurationRequest>::builder()
				.path(VerifyManagedURLConfigurationPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(VerifyManagedURLConfigurationRequestHeaders {
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
async fn create_managed_url_proxy_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	// `managed_url_fk_deployment_id_port` requires (deployment_id, port) to
	// exist in `deployment_exposed_port`, so the deployment must declare 8080.
	let machine_type = setup
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
	ports.insert(StringifiedU16::new(8080), ExposedPortType::Http);

	let deployment = setup
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
					machine_type,
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
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>()
		.response
		.id
		.id;

	let _ = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: rand_subdomain(),
					domain_id: domain.id,
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyDeployment {
						deployment_id: deployment,
						port: 8080,
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();
}

#[tokio::test]
async fn create_managed_url_proxy_url() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	let _ = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: rand_subdomain(),
					domain_id: domain.id,
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyUrl {
						url: "https://example.com/upstream".to_string(),
						http_only: false,
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();
}

#[tokio::test]
async fn create_managed_url_redirect() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	for permanent in [true, false] {
		let _ = setup
			.make_web_dashboard_call(
				ApiRequest::<CreateManagedURLRequest>::builder()
					.path(CreateManagedURLPath {
						workspace_id: workspace.id,
					})
					.headers(CreateManagedURLRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateManagedURLRequest {
						sub_domain: rand_subdomain(),
						domain_id: domain.id,
						path: "/".to_string(),
						url_type: ManagedUrlType::Redirect {
							url: "https://example.com".to_string(),
							permanent_redirect: permanent,
							http_only: false,
						},
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();
	}
}

#[tokio::test]
async fn create_managed_url_duplicate_returns_already_exists() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	let sub_domain = rand_subdomain();
	let body = CreateManagedURLRequest {
		sub_domain: sub_domain.clone(),
		domain_id: domain.id,
		path: "/".to_string(),
		url_type: ManagedUrlType::Redirect {
			url: "https://example.com".to_string(),
			permanent_redirect: false,
			http_only: false,
		},
	};

	let _ = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(body.clone())
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(body)
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"duplicate (sub_domain, domain_id, path) should be rejected with ResourceAlreadyExists, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn create_managed_url_invalid_deployment_id() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: rand_subdomain(),
					domain_id: domain.id,
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyDeployment {
						deployment_id: Uuid::nil(),
						port: 8080,
					},
				})
				.build(),
		)
		.await;

	assert_eq!(
		400,
		response.status_code().as_u16(),
		"managed URL pointing to a nonexistent deployment should be WrongParameters (400)"
	);
}

#[tokio::test]
async fn create_managed_url_unverified_domain() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	// Note: NO mark_test_domain_verified — this is the test.
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: rand_subdomain(),
					domain_id: domain.id,
					path: "/".to_string(),
					url_type: ManagedUrlType::Redirect {
						url: "https://example.com".to_string(),
						permanent_redirect: false,
						http_only: false,
					},
				})
				.build(),
		)
		.await;

	assert_eq!(
		412,
		response.status_code().as_u16(),
		"creating a managed URL on an unverified domain should fail with DomainNotVerified (412)"
	);
}

#[tokio::test]
async fn delete_managed_url_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: Uuid::nil(),
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleting a nonexistent managed URL should fail"
	);
}

#[tokio::test]
async fn managed_url_cross_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace_a.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace_a.id, domain.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace_b.id,
					managed_url_id: url_id,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"managed URL in workspace A should not be reachable via workspace B's path"
	);
}

#[tokio::test]
async fn update_managed_url_change_redirect_to_proxy_url() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyUrl {
						url: "https://upstream.example.com".to_string(),
						http_only: false,
					},
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateManagedURLResponse));

	let listed = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	let url = listed
		.response
		.urls
		.iter()
		.find(|u| u.id == url_id)
		.expect("updated managed URL should be in the list");
	assert!(matches!(url.url_type, ManagedUrlType::ProxyUrl { .. }));
}

#[tokio::test]
async fn update_managed_url_change_proxy_url_to_redirect() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	let create_resp = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: rand_subdomain(),
					domain_id: domain.id,
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyUrl {
						url: "https://upstream.example.com".to_string(),
						http_only: false,
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();
	let url_id = create_resp.response.id.id;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: "/".to_string(),
					url_type: ManagedUrlType::Redirect {
						url: "https://final.example.com".to_string(),
						permanent_redirect: true,
						http_only: false,
					},
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateManagedURLResponse));

	let listed = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	let url = listed
		.response
		.urls
		.iter()
		.find(|u| u.id == url_id)
		.expect("updated managed URL should be in the list");
	assert!(matches!(
		url.url_type,
		ManagedUrlType::Redirect {
			permanent_redirect: true,
			..
		}
	));
}

#[tokio::test]
async fn get_managed_url_info_via_list() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	let sub_domain = rand_subdomain();
	let create_resp = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: sub_domain.clone(),
					domain_id: domain.id,
					path: "/some/path".to_string(),
					url_type: ManagedUrlType::ProxyUrl {
						url: "https://upstream.example.com".to_string(),
						http_only: true,
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();
	let url_id = create_resp.response.id.id;

	let listed = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();

	let url = listed
		.response
		.urls
		.iter()
		.find(|u| u.id == url_id)
		.expect("created managed URL should appear in list");
	assert_eq!(url.sub_domain, sub_domain);
	assert_eq!(url.domain_id, domain.id);
	assert_eq!(url.path, "/some/path");
	match &url.url_type {
		ManagedUrlType::ProxyUrl { url: u, http_only } => {
			assert_eq!(u, "https://upstream.example.com");
			assert!(*http_only);
		}
		other => panic!("expected ProxyUrl, got {other:?}"),
	}
}

#[tokio::test]
async fn verify_configuration_not_configured() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	// Point this managed URL's custom hostname row at the wiremock id that
	// reports `status: "pending"` (see `mount_cloudflare_mocks` in setup.rs).
	setup
		.execute_sql(
			"UPDATE managed_url_custom_hostname SET cloudflare_custom_hostname_id = \
			 'pending-hostname-id' WHERE cloudflare_custom_hostname_id = 'mock-custom-hostname-id'",
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyManagedURLConfigurationRequest>::builder()
				.path(VerifyManagedURLConfigurationPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(VerifyManagedURLConfigurationRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<VerifyManagedURLConfigurationResponse>>();

	assert!(
		!response.response.configured,
		"hostname status `pending` should report configured = false"
	);
}

#[tokio::test]
async fn create_managed_url_apex_subdomain() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let resp = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 80, "@".to_string(), "/"),
	)
	.await;
	assert!(
		resp.status_code().is_success(),
		"the @ subdomain (apex) should be accepted, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_managed_url_nonexistent_domain_400() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let resp = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(Uuid::nil(), f.deployment, 80, rand_subdomain(), "/"),
	)
	.await;
	assert_eq!(
		400,
		resp.status_code().as_u16(),
		"a nonexistent domain should be WrongParameters (400)"
	);
}

#[tokio::test]
async fn create_managed_url_unexposed_port_500() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let resp = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 9999, rand_subdomain(), "/"),
	)
	.await;
	assert!(
		resp.status_code().is_server_error(),
		"a port the deployment doesn't expose should fail the FK → 500, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_managed_url_same_subdomain_different_path() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let sub = rand_subdomain();

	assert!(
		send_create_mu(
			&setup,
			&f.user.access_token,
			f.ws,
			proxy_body(f.domain, f.deployment, 80, sub.clone(), "/"),
		)
		.await
		.status_code()
		.is_success()
	);
	assert_eq!(
		409,
		send_create_mu(
			&setup,
			&f.user.access_token,
			f.ws,
			proxy_body(f.domain, f.deployment, 80, sub.clone(), "/"),
		)
		.await
		.status_code()
		.as_u16(),
		"duplicate (sub, domain, path) should be 409"
	);
	assert!(
		send_create_mu(
			&setup,
			&f.user.access_token,
			f.ws,
			proxy_body(f.domain, f.deployment, 80, sub, "/api"),
		)
		.await
		.status_code()
		.is_success(),
		"the same subdomain with a different path should be allowed"
	);
}

#[tokio::test]
async fn create_managed_url_lowercases_and_slash_prefixes() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let sub = rand_subdomain().to_uppercase();
	let created = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		CreateManagedURLRequest {
			sub_domain: sub.clone(),
			domain_id: f.domain,
			path: "API/V1".to_string(),
			url_type: ManagedUrlType::ProxyDeployment {
				deployment_id: f.deployment,
				port: 80,
			},
		},
	)
	.await
	.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
	.response;

	let urls = list_mus(&setup, &f.user.access_token, f.ws).await;
	let row = urls.iter().find(|u| u.id == created.id.id).expect("row");
	assert_eq!(row.sub_domain, sub.to_lowercase());
	assert_eq!(row.path, "/api/v1");
}

#[tokio::test]
async fn create_managed_url_shares_custom_hostname_row() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let sub = rand_subdomain();
	for path in ["/", "/api"] {
		send_create_mu(
			&setup,
			&f.user.access_token,
			f.ws,
			proxy_body(f.domain, f.deployment, 80, sub.clone(), path),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();
	}

	let count: i64 = sqlx::query_scalar(
		"SELECT count(*) FROM managed_url_custom_hostname WHERE sub_domain = $1 AND domain_id = $2",
	)
	.bind(&sub)
	.bind(f.domain)
	.fetch_one(setup.database())
	.await
	.expect("count query");
	assert_eq!(
		1, count,
		"two URLs on the same FQDN should share one custom-hostname row"
	);
}

#[tokio::test]
async fn create_managed_url_invalid_subdomain_500() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let resp = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 80, "has spaces!".to_string(), "/"),
	)
	.await;
	assert!(
		resp.status_code().is_server_error(),
		"an invalid subdomain should hit the DB CHECK → 500, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_managed_url_proxy_url_no_validation() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ws = setup.create_test_workspace(&user.access_token).await.id;
	let domain = verified_domain(&setup, &user.access_token, ws).await;

	// ProxyUrl `url` is not validated — a garbage value is accepted (gap).
	let resp = send_create_mu(
		&setup,
		&user.access_token,
		ws,
		CreateManagedURLRequest {
			sub_domain: rand_subdomain(),
			domain_id: domain,
			path: "/".to_string(),
			url_type: ManagedUrlType::ProxyUrl {
				url: "not a url at all".to_string(),
				http_only: false,
			},
		},
	)
	.await;
	assert!(
		resp.status_code().is_success(),
		"ProxyUrl accepts an unvalidated url, got {}",
		resp.status_code()
	);
}

#[tokio::test]
async fn create_managed_url_empty_path_normalized() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let created = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 80, rand_subdomain(), ""),
	)
	.await
	.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
	.response;

	let urls = list_mus(&setup, &f.user.access_token, f.ws).await;
	assert_eq!(
		urls.iter().find(|u| u.id == created.id.id).unwrap().path,
		"/",
		"an empty path should normalize to /"
	);
}

#[tokio::test]
async fn update_managed_url_path_persists() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let created = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 80, rand_subdomain(), "/"),
	)
	.await
	.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
	.response;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: f.ws,
					managed_url_id: created.id.id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: f.user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: "/v2".to_string(),
					url_type: ManagedUrlType::ProxyDeployment {
						deployment_id: f.deployment,
						port: 80,
					},
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateManagedURLResponse));

	let urls = list_mus(&setup, &f.user.access_token, f.ws).await;
	assert_eq!(
		urls.iter().find(|u| u.id == created.id.id).unwrap().path,
		"/v2"
	);
}

#[tokio::test]
async fn update_managed_url_same_values_noop() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let created = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 80, rand_subdomain(), "/"),
	)
	.await
	.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
	.response;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: f.ws,
					managed_url_id: created.id.id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: f.user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: "/".to_string(),
					url_type: ManagedUrlType::ProxyDeployment {
						deployment_id: f.deployment,
						port: 80,
					},
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateManagedURLResponse));

	let urls = list_mus(&setup, &f.user.access_token, f.ws).await;
	assert_eq!(
		urls.iter().find(|u| u.id == created.id.id).unwrap().path,
		"/",
		"resending the same values should be a no-op"
	);
}

#[tokio::test]
async fn update_managed_url_nonexistent_401() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ws = setup.create_test_workspace(&user.access_token).await.id;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: ws,
					managed_url_id: Uuid::nil(),
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: "/x".to_string(),
					url_type: ManagedUrlType::ProxyUrl {
						url: "https://example.com".to_string(),
						http_only: false,
					},
				})
				.build(),
		)
		.await;
	assert_eq!(
		401,
		response.status_code().as_u16(),
		"update on a nonexistent managed URL should 401 (anti-enumeration)"
	);
}

#[tokio::test]
async fn verify_configuration_active_flips_is_active() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	let created = send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 80, rand_subdomain(), "/"),
	)
	.await
	.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
	.response;

	let verify = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyManagedURLConfigurationRequest>::builder()
				.path(VerifyManagedURLConfigurationPath {
					workspace_id: f.ws,
					managed_url_id: created.id.id,
				})
				.headers(VerifyManagedURLConfigurationRequestHeaders {
					authorization: f.user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<VerifyManagedURLConfigurationResponse>>();
	assert!(
		verify.response.configured,
		"an active custom hostname should report configured = true"
	);

	let urls = list_mus(&setup, &f.user.access_token, f.ws).await;
	assert!(
		urls.iter()
			.find(|u| u.id == created.id.id)
			.unwrap()
			.is_active,
		"verify should flip is_active to true"
	);
}

#[tokio::test]
async fn verify_configuration_nonexistent_401() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let ws = setup.create_test_workspace(&user.access_token).await.id;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyManagedURLConfigurationRequest>::builder()
				.path(VerifyManagedURLConfigurationPath {
					workspace_id: ws,
					managed_url_id: Uuid::nil(),
				})
				.headers(VerifyManagedURLConfigurationRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		401,
		response.status_code().as_u16(),
		"verify on a nonexistent managed URL should 401"
	);
}

#[tokio::test]
async fn list_managed_urls_ordered_created_desc() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;

	let mut ids = Vec::new();
	for _ in 0..3 {
		ids.push(
			send_create_mu(
				&setup,
				&f.user.access_token,
				f.ws,
				proxy_body(f.domain, f.deployment, 80, rand_subdomain(), "/"),
			)
			.await
			.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>()
			.response
			.id
			.id,
		);
	}

	let listed: Vec<Uuid> = list_mus(&setup, &f.user.access_token, f.ws)
		.await
		.iter()
		.map(|u| u.id)
		.collect();
	ids.reverse();
	assert_eq!(ids, listed, "managed URLs should be ordered created DESC");
}

#[tokio::test]
async fn list_managed_urls_page_out_of_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let f = mu_fixture(&setup).await;
	send_create_mu(
		&setup,
		&f.user.access_token,
		f.ws,
		proxy_body(f.domain, f.deployment, 80, rand_subdomain(), "/"),
	)
	.await
	.json::<ApiSuccessResponseBody<CreateManagedURLResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath { workspace_id: f.ws })
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 10,
					page: 50,
					additional_query: (),
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: f.user.access_token.clone(),
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
async fn managed_url_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}
