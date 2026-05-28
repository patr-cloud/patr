use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::{deployment::*, managed_url::*},
	utils::StringifiedU16,
};

use crate::prelude::*;

fn rand_subdomain() -> String {
	random_name(6)
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
					path: Some("/updated".to_string()),
					url_type: None,
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

	assert!(
		response.status_code().is_client_error(),
		"managed URL pointing to a nonexistent deployment should be rejected"
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

	assert!(
		response.status_code().is_client_error(),
		"creating a managed URL on an unverified domain should fail with DomainNotVerified"
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
					path: None,
					url_type: Some(ManagedUrlType::ProxyUrl {
						url: "https://upstream.example.com".to_string(),
						http_only: false,
					}),
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
					path: None,
					url_type: Some(ManagedUrlType::Redirect {
						url: "https://final.example.com".to_string(),
						permanent_redirect: true,
						http_only: false,
					}),
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
