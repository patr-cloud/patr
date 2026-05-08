use std::collections::BTreeMap;

use api::routes::registry_patr_cloud::handlers::{blob::*, manifest::*};
use models::{
	ApiSuccessResponseBody,
	api::workspace::container_registry::*,
	rbac::WorkspacePermission,
};

use super::helpers::*;
use crate::prelude::*;

#[tokio::test]
async fn push_and_pull_image() {
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
		.push_test_image(&api_token.token, &workspace.id, &repo.name, "latest")
		.await;

	// Pull manifest back
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetManifestPath> {
			path: GetManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: "latest".to_string(),
			},
			query: (),
			headers: GetManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	assert_eq!(
		response.into_bytes().as_ref(),
		image.manifest_bytes.as_slice()
	);

	// Pull layer blob back
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: image.layer_digest.clone(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	assert_eq!(response.into_bytes().as_ref(), image.layer_bytes.as_slice());

	// Pull config blob back
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: image.config_digest.clone(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	assert_eq!(
		response.into_bytes().as_ref(),
		image.config_bytes.as_slice()
	);
}

#[tokio::test]
async fn push_image_shows_in_api_manifests() {
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

	// Check via API that the manifest appears
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

	assert!(
		!response.response.manifests.is_empty(),
		"expected at least one manifest after push"
	);

	let manifest = response
		.response
		.manifests
		.iter()
		.find(|m| m.digest == image.manifest_digest);
	assert!(
		manifest.is_some(),
		"pushed manifest digest {} not found in API list",
		image.manifest_digest
	);
}

#[tokio::test]
async fn push_tag_shows_in_api_tags() {
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

	// Check via API that the tag appears
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

	let tag = response.response.tags.iter().find(|t| t.tag == "v1");
	assert!(tag.is_some(), "pushed tag 'v1' not found in API tag list");

	let tag = tag.unwrap();
	assert_eq!(
		tag.digest, image.manifest_digest,
		"tag digest does not match pushed manifest digest"
	);
}

#[tokio::test]
async fn push_tag_updates_existing() {
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

	// Push first image with tag "v1" (seed 0)
	let image1 = setup
		.push_test_image(&api_token.token, &workspace.id, &repo.name, "v1")
		.await;

	// Push a different image with the same tag "v1" (seed 42)
	let image2 = {
		let img = build_minimal_oci_image(42);
		setup
			.push_blob(
				&api_token.token,
				&workspace.id,
				&repo.name,
				&img.config_digest,
				&img.config_bytes,
			)
			.await;
		setup
			.push_blob(
				&api_token.token,
				&workspace.id,
				&repo.name,
				&img.layer_digest,
				&img.layer_bytes,
			)
			.await;
		let response = setup
			.push_manifest(
				&api_token.token,
				&workspace.id,
				&repo.name,
				"v1",
				&img.manifest_bytes,
			)
			.await;
		assert_eq!(
			response.status_code(),
			StatusCode::CREATED,
			"manifest push failed: {}",
			std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
		);
		img
	};

	// The two images must have different digests
	assert_ne!(
		image1.manifest_digest, image2.manifest_digest,
		"images with different seeds must produce different digests"
	);

	// Verify the tag now points to the second image
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

	let v1_tags: Vec<_> = response
		.response
		.tags
		.iter()
		.filter(|t| t.tag == "v1")
		.collect();

	assert_eq!(v1_tags.len(), 1, "expected exactly one 'v1' tag");
	assert_eq!(
		v1_tags[0].digest, image2.manifest_digest,
		"tag should point to the latest pushed image"
	);
}

#[tokio::test]
async fn delete_manifest_removes_from_list() {
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

	// Delete via API
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

	// Verify manifest is gone from list
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

	let found = response
		.response
		.manifests
		.iter()
		.any(|m| m.digest == image.manifest_digest);
	assert!(!found, "manifest should be gone after deletion");
}

#[tokio::test]
async fn delete_manifest_with_tag_removes_both() {
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

	// Delete manifest via API (by digest)
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

	// Verify tag is also gone
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

	let tag = response.response.tags.iter().find(|t| t.tag == "v1");
	assert!(
		tag.is_none(),
		"tag 'v1' should be removed after manifest deletion"
	);
}

#[tokio::test]
async fn registry_delete_manifest_returns_405() {
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

	// Try to DELETE via registry endpoint — should be 405
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<DeleteManifestPath> {
			path: DeleteManifestPath {
				repo_name: repo.name.clone(),
				reference: image.manifest_digest.clone(),
			},
			query: (),
			headers: DeleteManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::METHOD_NOT_ALLOWED,
		"registry DELETE manifest should return 405, got {}",
		response.status_code()
	);
}
