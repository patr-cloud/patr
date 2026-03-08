use std::collections::BTreeMap;

use api::routes::registry_patr_cloud::handlers::manifest::*;
use models::rbac::WorkspacePermission;

use super::helpers::*;
use crate::prelude::*;

#[tokio::test]
async fn push_manifest_with_tag() {
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

	// Verify digest is correct
	assert!(image.manifest_digest.starts_with("sha256:"));
}

#[tokio::test]
async fn get_manifest_by_tag() {
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
		.make_registry_call(RegistryUnprocessedApiRequest::<GetManifestPath> {
			path: GetManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: "v1".to_string(),
			},
			query: (),
			headers: GetManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);

	let digest_header = response
		.maybe_header("docker-content-digest")
		.expect("expected Docker-Content-Digest header");
	assert_eq!(digest_header.to_str().unwrap(), image.manifest_digest);

	let body = response.into_bytes();
	assert_eq!(body.as_ref(), image.manifest_bytes.as_slice());
}

#[tokio::test]
async fn get_manifest_by_digest() {
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
		.make_registry_call(RegistryUnprocessedApiRequest::<GetManifestPath> {
			path: GetManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: image.manifest_digest.clone(),
			},
			query: (),
			headers: GetManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);

	let body = response.into_bytes();
	assert_eq!(body.as_ref(), image.manifest_bytes.as_slice());
}

#[tokio::test]
async fn head_manifest_works() {
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
		.make_registry_call(RegistryUnprocessedApiRequest::<HeadManifestPath> {
			path: HeadManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: "v1".to_string(),
			},
			query: (),
			headers: HeadManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	assert_eq!(
		response
			.maybe_header("docker-content-digest")
			.expect("expected Docker-Content-Digest")
			.to_str()
			.unwrap(),
		image.manifest_digest
	);
	assert!(response.maybe_header("content-length").is_some());
}

#[tokio::test]
async fn get_manifest_nonexistent() {
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

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetManifestPath> {
			path: GetManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: "nonexistent-tag".to_string(),
			},
			query: (),
			headers: GetManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn push_manifest_unsupported_content_type() {
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

	let image = build_minimal_oci_image(0);

	// Push blobs first so they exist
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

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<PutManifestPath> {
			path: PutManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: "latest".to_string(),
			},
			query: (),
			headers: PutManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: {
					let mut map = http::HeaderMap::new();
					map.insert(http::header::CONTENT_TYPE, "text/plain".parse().unwrap());
					headers::HeaderMapExt::typed_get(&map).unwrap()
				},
			},
			body: Body::from(image.manifest_bytes),
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for unsupported content type, got {}",
		response.status_code()
	);
}
