use std::collections::{BTreeMap, BTreeSet};

use api::routes::registry_patr_cloud::handlers::{blob::*, manifest::*};
use headers::{ContentLength, ContentType};
use models::{
	api::{user::PermissionGrant, workspace::container_registry::*},
	rbac::{
		ContainerRegistryRepositoryPermission,
		DeploymentPermission,
		Permission,
		WorkspacePermission,
	},
};

use super::helpers::*;
use crate::prelude::*;

/// An anonymous `GET /v2/` (no Authorization header) is rejected with 401, a
/// `Bearer realm=` challenge, and the OCI `UNAUTHORIZED` error code.
#[tokio::test]
async fn registry_v2_anonymous_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_registry_raw_call(http::Method::GET, "/v2/", vec![], vec![])
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"anonymous GET /v2/ should be 401"
	);
	let challenge = response
		.maybe_header("www-authenticate")
		.expect("expected a WWW-Authenticate header on the anonymous request");
	assert!(
		challenge
			.to_str()
			.unwrap()
			.to_lowercase()
			.contains("bearer realm="),
		"expected a Bearer challenge, got {challenge:?}"
	);
	let body = response.json::<serde_json::Value>();
	assert_eq!(
		body["errors"][0]["code"].as_str(),
		Some("UNAUTHORIZED"),
		"expected the OCI UNAUTHORIZED error code"
	);
}

#[tokio::test]
async fn registry_push_without_permission() {
	let setup = setup().await.expect("failed to setup test server");

	// Admin creates workspace + repo
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

	// Second user (not in admin's workspace) gets an API token for their own
	// workspace
	let other = setup.create_test_user().await;
	let other_workspace = setup.create_test_workspace(&other.access_token).await;
	let other_token = setup
		.create_test_api_token(
			&other.access_token,
			BTreeSet::from([other_workspace.id]),
			BTreeMap::new(),
		)
		.await;

	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
			path: InitiateBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
			},
			query: InitiateBlobUploadQuery {
				mount: None,
				from: None,
				digest: Some(digest),
			},
			headers: InitiateBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&other_token.token).unwrap(),
				content_length: OptionalHeader::new(Some(ContentLength(data.len() as u64))),
				content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
			},
			body: Body::from(data),
		})
		.await;

	// Should be 404 (not 403) to avoid leaking repo existence
	assert_eq!(
		response.status_code(),
		StatusCode::NOT_FOUND,
		"expected 404 for push without permission, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn push_to_nonexistent_repo() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
			path: InitiateBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: "nonexistent-repo".to_string(),
			},
			query: InitiateBlobUploadQuery {
				mount: None,
				from: None,
				digest: Some(digest),
			},
			headers: InitiateBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_length: OptionalHeader::new(Some(ContentLength(data.len() as u64))),
				content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
			},
			body: Body::from(data),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pull_from_nonexistent_repo() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetManifestPath> {
			path: GetManifestPath {
				workspace_id: workspace.id,
				repo_name: "nonexistent-repo".to_string(),
				reference: "latest".to_string(),
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

/// A syntactically-invalid manifest reference (leading dot) on an existing repo
/// must 404 (ManifestUnknown), not 400 — matching the OCI conformance suite and
/// mainstream registries. Previously the `reference` regex rejected it at the
/// preprocess layer with a 400 before the handler's 404 path.
#[tokio::test]
async fn get_manifest_with_invalid_reference_returns_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetManifestPath> {
			path: GetManifestPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				reference: ".INVALID_MANIFEST_NAME".to_string(),
			},
			query: (),
			headers: GetManifestRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::NOT_FOUND,
		"invalid manifest reference should 404, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn push_to_deleted_repo() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	// Delete the repo via API
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

	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
			path: InitiateBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
			},
			query: InitiateBlobUploadQuery {
				mount: None,
				from: None,
				digest: Some(digest),
			},
			headers: InitiateBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_length: OptionalHeader::new(Some(ContentLength(data.len() as u64))),
				content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
			},
			body: Body::from(data),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::NOT_FOUND,
		"expected 404 for push to deleted repo"
	);
}

#[tokio::test]
async fn cross_workspace_push_denied() {
	let setup = setup().await.expect("failed to setup test server");

	// User A creates workspace A + repo
	let user_a = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user_a.access_token).await;
	let repo_a = setup
		.create_test_container_repo(&user_a.access_token, workspace_a.id)
		.await;

	// User B creates their own workspace and API token
	let user_b = setup.create_test_user().await;
	let workspace_b = setup.create_test_workspace(&user_b.access_token).await;
	let token_b = setup
		.create_test_api_token(
			&user_b.access_token,
			BTreeSet::from([workspace_b.id]),
			BTreeMap::new(),
		)
		.await;

	// User B tries to push to user A's repo
	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
			path: InitiateBlobUploadPath {
				workspace_id: workspace_a.id,
				repo_name: repo_a.name.clone(),
			},
			query: InitiateBlobUploadQuery {
				mount: None,
				from: None,
				digest: Some(digest),
			},
			headers: InitiateBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&token_b.token).unwrap(),
				content_length: OptionalHeader::new(Some(ContentLength(data.len() as u64))),
				content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
			},
			body: Body::from(data),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::NOT_FOUND,
		"expected 404 for cross-workspace push, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn initiate_upload_as_member_without_push_returns_forbidden() {
	let setup = setup().await.expect("failed to setup test server");

	// Admin creates workspace + repo
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

	// Create a role with only Deployment::View (NOT
	// ContainerRegistryRepository::Push)
	let perm_id = setup.get_permission_id(Permission::Deployment(DeploymentPermission::View));
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, vec![perm_id])
		.await;

	// Add a second user to the workspace with that limited role
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// Second user creates an API token (no workspace-level SuperAdmin on token;
	// permissions come from their role membership)
	let token_b = setup
		.create_test_api_token(
			&user_b.access_token,
			BTreeSet::new(),
			BTreeMap::from([(
				workspace.id,
				vec![PermissionGrant {
					permission_id: perm_id,
					resource_id: workspace.id,
				}],
			)]),
		)
		.await;

	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
			path: InitiateBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
			},
			query: InitiateBlobUploadQuery {
				mount: None,
				from: None,
				digest: Some(digest),
			},
			headers: InitiateBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&token_b.token).unwrap(),
				content_length: OptionalHeader::new(Some(ContentLength(data.len() as u64))),
				content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
			},
			body: Body::from(data),
		})
		.await;

	// A workspace member who lacks Push gets a clear 403 — the existence-hiding
	// 404 is reserved for non-members (who can't already list the repo).
	let status = response.status_code();
	let body = String::from_utf8_lossy(&response.into_bytes()).to_string();
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"expected 403 for member without push permission, got {status}"
	);
	assert!(
		body.contains("push access"),
		"expected a push-access denial message, got: {body}"
	);
}

/// A `docker push` HEADs each blob to check existence before uploading, so a
/// push-only token must be allowed to run that read. Regression: the pull route
/// checked only Pull, so a push-only member was denied the existence check and
/// the push broke. The pull route now accepts push OR pull.
#[tokio::test]
async fn head_blob_with_push_only_token_is_allowed() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	// A token that is a workspace member with ONLY push (no pull).
	let push_perm = setup.get_permission_id(Permission::ContainerRegistryRepository(
		ContainerRegistryRepositoryPermission::Push,
	));
	let push_role = setup
		.create_role_with_permissions(&user.access_token, workspace.id, vec![push_perm])
		.await;
	let push_only = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::new(),
			BTreeMap::from([(
				workspace.id,
				vec![PermissionGrant {
					permission_id: push_perm,
					resource_id: workspace.id,
				}],
			)]),
		)
		.await;

	// HEAD a (nonexistent) blob — the existence check a push does first.
	let digest = sha256_digest(&(0..64u8).collect::<Vec<u8>>());
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<HeadBlobPath> {
			path: HeadBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest,
			},
			query: (),
			headers: HeadBlobRequestHeaders {
				authorization: BearerToken::from_str(&push_only.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	// Must not be denied: a push-capable token can check blob existence. The
	// blob doesn't exist, so 404 is the correct answer.
	assert_ne!(
		response.status_code(),
		StatusCode::FORBIDDEN,
		"push-only token was denied the blob existence check needed for push"
	);
	assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}
