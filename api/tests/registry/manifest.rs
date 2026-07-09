use std::collections::BTreeMap;

use api::routes::registry_patr_cloud::handlers::manifest::*;
use models::rbac::WorkspacePermission;

use super::helpers::*;
use crate::prelude::*;

/// Native OCI manifest deletion is refused — deletion goes through the Patr
/// API, so a raw `DELETE /v2/.../manifests/{ref}` is a client error.
#[tokio::test]
async fn native_manifest_delete_refused() {
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

	let auth = format!("Bearer {}", api_token.token);
	let path = format!(
		"/v2/{}/{}/manifests/sha256:{}",
		workspace.id,
		repo.name,
		"0".repeat(64)
	);
	let response = setup
		.make_registry_raw_call(
			http::Method::DELETE,
			&path,
			vec![(http::header::AUTHORIZATION, auth.as_str())],
			vec![],
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"native OCI manifest DELETE should be refused, got {}",
		response.status_code()
	);
}

/// A supported content-type with an unparseable manifest body is rejected with
/// 400 (the body fails to deserialize into an OCI image manifest).
#[tokio::test]
async fn push_manifest_malformed_body() {
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
		.push_manifest(
			&api_token.token,
			&workspace.id,
			&repo.name,
			"sometag",
			br#"{"not":"a valid manifest"}"#,
		)
		.await;

	assert_eq!(
		400,
		response.status_code().as_u16(),
		"a malformed manifest body should be rejected with 400"
	);
}

/// Pushing a manifest to a nonexistent repo is 404 with the OCI `NAME_UNKNOWN`
/// error code (no auto-create).
#[tokio::test]
async fn push_manifest_to_nonexistent_repo() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let image = build_minimal_oci_image(0);
	let response = setup
		.push_manifest(
			&api_token.token,
			&workspace.id,
			"does-not-exist",
			"v1",
			&image.manifest_bytes,
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
	let body = response.json::<serde_json::Value>();
	assert_eq!(
		body["errors"][0]["code"].as_str(),
		Some("NAME_UNKNOWN"),
		"expected the OCI NAME_UNKNOWN error code"
	);
}

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

/// Pushing an OCI image index (manifest list) works end to end. docker 29's
/// containerd path pushes an index even for single-arch images; the index row
/// stores NULL `config_blob_digest`/`platform`. Regression test for the NOT
/// NULL constraints that used to fail every index push with a 500.
#[tokio::test]
async fn push_index_manifest_works() {
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

	// Push a regular single-arch image first; the index references it.
	let image = setup
		.push_test_image(&api_token.token, &workspace.id, &repo.name, "amd64")
		.await;

	let index = serde_json::json!({
		"schemaVersion": 2,
		"mediaType": "application/vnd.oci.image.index.v1+json",
		"manifests": [{
			"mediaType": "application/vnd.oci.image.manifest.v1+json",
			"digest": image.manifest_digest,
			"size": image.manifest_bytes.len(),
			"platform": { "architecture": "amd64", "os": "linux" }
		}]
	});
	let index_bytes = serde_json::to_vec(&index).unwrap();

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<PutManifestPath> {
			path: PutManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: "multi".to_string(),
			},
			query: (),
			headers: PutManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: {
					let mut map = http::HeaderMap::new();
					map.insert(
						http::header::CONTENT_TYPE,
						"application/vnd.oci.image.index.v1+json".parse().unwrap(),
					);
					headers::HeaderMapExt::typed_get(&map).unwrap()
				},
			},
			body: Body::from(index_bytes.clone()),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::CREATED,
		"index manifest push failed: {}",
		std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// Pull the index back by tag and confirm it round-trips.
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetManifestPath> {
			path: GetManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: "multi".to_string(),
			},
			query: (),
			headers: GetManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	assert_eq!(response.into_bytes().as_ref(), index_bytes.as_slice());
}
