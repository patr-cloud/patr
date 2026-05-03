use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::{container_registry::*, deployment::*},
	utils::Uuid,
};

use crate::prelude::*;

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
