use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::container_registry::*,
	},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn create_repository_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let repo =
		create_test_container_repo(&setup, &user.access_token, ws.id).await;
	assert!(!repo.name.is_empty());
}

#[tokio::test]
async fn create_repository_duplicate() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let repo =
		create_test_container_repo(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			CreateContainerRepositoryRequest::METHOD,
			&CreateContainerRepositoryPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&CreateContainerRepositoryRequest {
			name: repo.name.clone(),
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate repository name"
	);
}

#[tokio::test]
async fn list_repositories_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let _repo =
		create_test_container_repo(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			ListContainerRepositoriesRequest::METHOD,
			&ListContainerRepositoriesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoriesResponse>>();

	assert_eq!(1, response.response.repositories.len());
}

#[tokio::test]
async fn list_repositories_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListContainerRepositoriesRequest::METHOD,
			&ListContainerRepositoriesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoriesResponse>>();

	assert!(response.response.repositories.is_empty());
}

#[tokio::test]
async fn get_repository_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let repo =
		create_test_container_repo(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			GetContainerRepositoryInfoRequest::METHOD,
			&GetContainerRepositoryInfoPath {
				workspace_id: ws.id,
				repository_id: repo.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetContainerRepositoryInfoResponse>>();

	assert_eq!(repo.name, response.response.repository.name);
}

#[tokio::test]
async fn get_repository_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetContainerRepositoryInfoRequest::METHOD,
			&GetContainerRepositoryInfoPath {
				workspace_id: ws.id,
				repository_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn delete_repository_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let repo =
		create_test_container_repo(&setup, &user.access_token, ws.id).await;

	setup
		.server
		.method(
			DeleteContainerRepositoryRequest::METHOD,
			&DeleteContainerRepositoryPath {
				workspace_id: ws.id,
				repository_id: repo.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteContainerRepositoryResponse,
		));

	// Verify it's gone
	let response = setup
		.server
		.method(
			GetContainerRepositoryInfoRequest::METHOD,
			&GetContainerRepositoryInfoPath {
				workspace_id: ws.id,
				repository_id: repo.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_manifests_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let repo =
		create_test_container_repo(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			ListContainerRepositoryManifestsRequest::METHOD,
			&ListContainerRepositoryManifestsPath {
				workspace_id: ws.id,
				repository_id: repo.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoryManifestsResponse>>();

	assert!(response.response.manifests.is_empty());
}

#[tokio::test]
async fn list_tags_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let repo =
		create_test_container_repo(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			ListContainerRepositoryTagsRequest::METHOD,
			&ListContainerRepositoryTagsPath {
				workspace_id: ws.id,
				repository_id: repo.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoryTagsResponse>>();

	assert!(response.response.tags.is_empty());
}

#[tokio::test]
async fn container_registry_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListContainerRepositoriesRequest::METHOD,
			&ListContainerRepositoriesPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(response.status_code().is_client_error());
}
