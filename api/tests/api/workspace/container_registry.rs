use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::{container_registry::*, deployment::*},
	rbac::WorkspacePermission,
	utils::Uuid,
};

use crate::{prelude::*, registry::helpers::build_minimal_oci_image_with_ports};

#[tokio::test]
async fn create_repository_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	assert!(!repo.name.is_empty());
}

#[tokio::test]
async fn create_repository_duplicate() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateContainerRepositoryRequest>::builder()
				.path(CreateContainerRepositoryPath {
					workspace_id: workspace.id,
				})
				.headers(CreateContainerRepositoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateContainerRepositoryRequest {
					name: repo.name.clone(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate repository name"
	);
}

#[tokio::test]
async fn list_repositories_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath {
					workspace_id: workspace.id,
				})
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoriesResponse>>();

	assert_eq!(1, response.response.repositories.len());
}

#[tokio::test]
async fn list_repositories_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath {
					workspace_id: workspace.id,
				})
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoriesResponse>>();

	assert!(response.response.repositories.is_empty());
}

#[tokio::test]
async fn get_repository_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetContainerRepositoryInfoResponse>>();

	assert_eq!(repo.name, response.response.repository.name);
}

#[tokio::test]
async fn get_repository_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: Uuid::nil(),
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn delete_repository_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryRequest>::builder()
				.path(DeleteContainerRepositoryPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(DeleteContainerRepositoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteContainerRepositoryResponse,
		));

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_manifests_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoryManifestsRequest>::builder()
				.path(ListContainerRepositoryManifestsPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(ListContainerRepositoryManifestsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoryManifestsResponse>>();

	assert!(response.response.manifests.is_empty());
}

#[tokio::test]
async fn list_tags_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoryTagsRequest>::builder()
				.path(ListContainerRepositoryTagsPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(ListContainerRepositoryTagsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoryTagsResponse>>();

	assert!(response.response.tags.is_empty());
}

#[tokio::test]
async fn create_repository_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateContainerRepositoryRequest>::builder()
				.path(CreateContainerRepositoryPath {
					workspace_id: workspace.id,
				})
				.headers(CreateContainerRepositoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateContainerRepositoryRequest {
					name: "!!!".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"repository name failing RESOURCE_NAME_REGEX should be rejected"
	);
}

#[tokio::test]
async fn delete_repository_in_use_by_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

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

	// Create a deployment that points at the patr-registry repo.
	let _ = setup
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
					registry: DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: repo.id,
					},
					image_tag: "latest".to_string(),
					runner: runner.id,
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
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryRequest>::builder()
				.path(DeleteContainerRepositoryPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(DeleteContainerRepositoryRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleting a repository referenced by a deployment should fail with ResourceInUse"
	);
}

#[tokio::test]
async fn container_registry_cross_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace_a.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace_b.id,
					repository_id: repo.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"repository in workspace A should not be reachable via workspace B's path"
	);
}

#[tokio::test]
async fn get_manifest_details_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	let image = setup
		.push_test_image(&api_token.token, &workspace.id, &repo.name, "v1")
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryManifestDetailsRequest>::builder()
				.path(GetContainerRepositoryManifestDetailsPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
					digest_or_tag: image.manifest_digest.clone(),
				})
				.headers(GetContainerRepositoryManifestDetailsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetContainerRepositoryManifestDetailsResponse>>();

	assert_eq!(image.manifest_digest, response.response.manifest_details.digest);
	assert!(response.response.manifest_details.tags.iter().any(|t| t == "v1"));
}

#[tokio::test]
async fn get_manifest_details_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryManifestDetailsRequest>::builder()
				.path(GetContainerRepositoryManifestDetailsPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
					digest_or_tag: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
						.to_string(),
				})
				.headers(GetContainerRepositoryManifestDetailsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"unknown manifest digest should return 4xx"
	);
}

#[tokio::test]
async fn delete_manifest_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	let image = setup
		.push_test_image(&api_token.token, &workspace.id, &repo.name, "v1")
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryManifestRequest>::builder()
				.path(DeleteContainerRepositoryManifestPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
					digest_or_tag: image.manifest_digest.clone(),
				})
				.headers(DeleteContainerRepositoryManifestRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteContainerRepositoryManifestResponse,
		));

	let manifests = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoryManifestsRequest>::builder()
				.path(ListContainerRepositoryManifestsPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(ListContainerRepositoryManifestsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoryManifestsResponse>>();

	assert!(
		manifests.response.manifests.is_empty(),
		"manifest should be gone after delete"
	);
}

#[tokio::test]
async fn delete_manifest_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryManifestRequest>::builder()
				.path(DeleteContainerRepositoryManifestPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
					digest_or_tag: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
						.to_string(),
				})
				.headers(DeleteContainerRepositoryManifestRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleting an unknown manifest should return 4xx"
	);
}

#[tokio::test]
async fn get_exposed_ports_no_ports() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	setup
		.push_test_image(&api_token.token, &workspace.id, &repo.name, "v1")
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryExposedPortsRequest>::builder()
				.path(GetContainerRepositoryExposedPortsPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
					digest_or_tag: "v1".to_string(),
				})
				.headers(GetContainerRepositoryExposedPortsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetContainerRepositoryExposedPortsResponse>>();

	assert!(
		response.response.ports.is_empty(),
		"image with no ExposedPorts in config should return empty list"
	);
}

#[tokio::test]
async fn get_exposed_ports_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	// Custom push: image config declares 8080/tcp.
	let image = build_minimal_oci_image_with_ports(0, &[8080]);
	setup
		.push_blob(
			&api_token.token,
			&workspace.id,
			&repo.name,
			&image.config_digest,
			&image.config_bytes,
		)
		.await;
	setup
		.push_blob(
			&api_token.token,
			&workspace.id,
			&repo.name,
			&image.layer_digest,
			&image.layer_bytes,
		)
		.await;
	let push_resp = setup
		.push_manifest(
			&api_token.token,
			&workspace.id,
			&repo.name,
			"v1",
			&image.manifest_bytes,
		)
		.await;
	assert_eq!(push_resp.status_code(), StatusCode::CREATED);

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryExposedPortsRequest>::builder()
				.path(GetContainerRepositoryExposedPortsPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
					digest_or_tag: "v1".to_string(),
				})
				.headers(GetContainerRepositoryExposedPortsRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetContainerRepositoryExposedPortsResponse>>();

	assert_eq!(
		response.response.ports,
		vec![8080],
		"image with ExposedPorts: 8080/tcp should return [8080]"
	);
}

#[tokio::test]
async fn delete_manifest_in_use_by_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	let image = setup
		.push_test_image(&api_token.token, &workspace.id, &repo.name, "v1")
		.await;

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

	let _ = setup
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
					registry: DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: repo.id,
					},
					image_tag: "v1".to_string(),
					runner: runner.id,
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
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryManifestRequest>::builder()
				.path(DeleteContainerRepositoryManifestPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
					digest_or_tag: image.manifest_digest.clone(),
				})
				.headers(DeleteContainerRepositoryManifestRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleting a manifest used by a live deployment should fail with ResourceInUse"
	);
}

#[tokio::test]
async fn container_registry_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath {
					workspace_id: workspace.id,
				})
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}
