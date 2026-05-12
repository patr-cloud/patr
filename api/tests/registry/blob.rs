use std::collections::BTreeMap;

use api::routes::registry_patr_cloud::handlers::blob::*;
use headers::{ContentLength, ContentType};
use models::rbac::WorkspacePermission;

use super::helpers::*;
use crate::prelude::*;

#[tokio::test]
async fn monolithic_blob_upload_works() {
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

	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	setup
		.push_blob(&api_token.token, &workspace.id, &repo.name, &digest, &data)
		.await;
}

#[tokio::test]
async fn blob_upload_wrong_content_type() {
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
				content_length: ContentLength(data.len() as u64),
				content_type: OptionalHeader::new(Some(ContentType::json())),
			},
			body: Body::from(data),
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong content type, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn get_blob_after_upload() {
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

	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	setup
		.push_blob(&api_token.token, &workspace.id, &repo.name, &digest, &data)
		.await;

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: digest.clone(),
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
	assert_eq!(response.into_bytes().as_ref(), &data);
}

#[tokio::test]
async fn head_blob_after_upload() {
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

	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	setup
		.push_blob(&api_token.token, &workspace.id, &repo.name, &digest, &data)
		.await;

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<HeadBlobPath> {
			path: HeadBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: digest.clone(),
			},
			query: (),
			headers: HeadBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	assert!(response.maybe_header("docker-content-digest").is_some());
}

#[tokio::test]
async fn get_blob_nonexistent() {
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

	let fake_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: fake_digest.to_string(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn chunked_upload_initiate_works() {
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
		.make_registry_call(RegistryUnprocessedApiRequest::<InitiateBlobUploadPath> {
			path: InitiateBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
			},
			query: InitiateBlobUploadQuery {
				mount: None,
				from: None,
				digest: None,
			},
			headers: InitiateBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_length: ContentLength(0),
				content_type: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(response.status_code(), StatusCode::ACCEPTED);
	assert!(
		response.maybe_header("location").is_some(),
		"expected Location header on chunked upload initiation"
	);
	assert!(
		response.maybe_header("docker-upload-uuid").is_some(),
		"expected Docker-Upload-UUID header"
	);
}

#[tokio::test]
async fn chunked_upload_complete_works() {
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

	// Initiate chunked upload
	let (session_id, _) = setup
		.initiate_chunked_upload(&api_token.token, &workspace.id, &repo.name)
		.await;

	// Complete upload with PUT
	let data: Vec<u8> = (0..64u8).collect();
	let digest = sha256_digest(&data);

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<CompleteBlobUploadPath> {
			path: CompleteBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				session_id,
			},
			query: CompleteBlobUploadQuery {
				digest: digest.clone(),
			},
			headers: CompleteBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
				content_length: OptionalHeader::new(Some(ContentLength(data.len() as u64))),
				content_range: OptionalHeader::new(None),
			},
			body: Body::from(data),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::CREATED,
		"chunked upload complete failed: {}",
		std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
	);
}

#[tokio::test]
async fn chunked_upload_with_patch_under_threshold() {
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

	// Initiate chunked upload
	let (session_id, _) = setup
		.initiate_chunked_upload(&api_token.token, &workspace.id, &repo.name)
		.await;

	// PATCH with ~1KB data (under 5MB — goes to Redis pending buffer)
	let data: Vec<u8> = (0..=255u8).cycle().take(1024).collect();
	let digest = sha256_digest(&data);

	let patch_response = setup
		.patch_blob_chunk(
			&api_token.token,
			&workspace.id,
			&repo.name,
			session_id,
			&data,
		)
		.await;
	assert_eq!(
		patch_response.status_code(),
		StatusCode::ACCEPTED,
		"PATCH chunk failed: {}",
		std::str::from_utf8(&patch_response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// PUT to complete with empty body + digest
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<CompleteBlobUploadPath> {
			path: CompleteBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				session_id,
			},
			query: CompleteBlobUploadQuery {
				digest: digest.clone(),
			},
			headers: CompleteBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: OptionalHeader::new(None),
				content_length: OptionalHeader::new(None),
				content_range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::CREATED,
		"complete upload failed: {}",
		std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// Verify blob GET returns the data
	let get_response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: digest.clone(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(get_response.status_code(), StatusCode::OK);
	assert_eq!(get_response.into_bytes().as_ref(), &data);

	setup.assert_blob_size_in_db(&digest, data.len() as u64).await;
}

#[tokio::test]
async fn chunked_upload_with_patch_over_threshold() {
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

	// Initiate chunked upload
	let (session_id, _) = setup
		.initiate_chunked_upload(&api_token.token, &workspace.id, &repo.name)
		.await;

	// PATCH with 5MB + 100 bytes (5MB flushes to S3, 100 bytes buffer in Redis)
	let size = 5 * 1024 * 1024 + 100;
	let data: Vec<u8> = (0..=255u8).cycle().take(size).collect();
	let digest = sha256_digest(&data);

	let patch_response = setup
		.patch_blob_chunk(
			&api_token.token,
			&workspace.id,
			&repo.name,
			session_id,
			&data,
		)
		.await;
	assert_eq!(
		patch_response.status_code(),
		StatusCode::ACCEPTED,
		"PATCH chunk failed: {}",
		std::str::from_utf8(&patch_response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// PUT to complete with empty body + digest
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<CompleteBlobUploadPath> {
			path: CompleteBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				session_id,
			},
			query: CompleteBlobUploadQuery {
				digest: digest.clone(),
			},
			headers: CompleteBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: OptionalHeader::new(None),
				content_length: OptionalHeader::new(None),
				content_range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::CREATED,
		"complete upload failed: {}",
		std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// Verify blob GET returns all 5MB+100 bytes
	let get_response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: digest.clone(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(get_response.status_code(), StatusCode::OK);
	assert_eq!(get_response.into_bytes().len(), size);

	setup.assert_blob_size_in_db(&digest, size as u64).await;
}

#[tokio::test]
async fn chunked_upload_multiple_patches() {
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

	// Initiate chunked upload
	let (session_id, _) = setup
		.initiate_chunked_upload(&api_token.token, &workspace.id, &repo.name)
		.await;

	// Build the full data for digest calculation
	let part1_size = 5 * 1024 * 1024; // exactly 5MB — flushes to S3
	let part2_size = 2 * 1024 * 1024; // 2MB — goes to Redis pending buffer
	let part1: Vec<u8> = (0..=255u8).cycle().take(part1_size).collect();
	let part2: Vec<u8> = (128..=255u8)
		.chain(0..=127u8)
		.cycle()
		.take(part2_size)
		.collect();

	let mut full_data = Vec::with_capacity(part1_size + part2_size);
	full_data.extend_from_slice(&part1);
	full_data.extend_from_slice(&part2);
	let digest = sha256_digest(&full_data);

	// PATCH with 5MB data (flushes to S3 as part 1)
	let patch1 = setup
		.patch_blob_chunk(
			&api_token.token,
			&workspace.id,
			&repo.name,
			session_id,
			&part1,
		)
		.await;
	assert_eq!(
		patch1.status_code(),
		StatusCode::ACCEPTED,
		"PATCH part 1 failed: {}",
		std::str::from_utf8(&patch1.into_bytes()).unwrap_or("<non-utf8>")
	);

	// PATCH with 2MB data (goes to Redis pending buffer)
	let patch2 = setup
		.patch_blob_chunk(
			&api_token.token,
			&workspace.id,
			&repo.name,
			session_id,
			&part2,
		)
		.await;
	assert_eq!(
		patch2.status_code(),
		StatusCode::ACCEPTED,
		"PATCH part 2 failed: {}",
		std::str::from_utf8(&patch2.into_bytes()).unwrap_or("<non-utf8>")
	);

	// PUT to complete with empty body + digest
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<CompleteBlobUploadPath> {
			path: CompleteBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				session_id,
			},
			query: CompleteBlobUploadQuery {
				digest: digest.clone(),
			},
			headers: CompleteBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: OptionalHeader::new(None),
				content_length: OptionalHeader::new(None),
				content_range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::CREATED,
		"complete upload failed: {}",
		std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// 5. Verify blob GET returns all 7MB
	let get_response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: digest.clone(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(get_response.status_code(), StatusCode::OK);
	assert_eq!(get_response.into_bytes().len(), part1_size + part2_size);

	setup
		.assert_blob_size_in_db(&digest, (part1_size + part2_size) as u64)
		.await;
}

#[tokio::test]
async fn chunked_upload_patch_then_body_in_put() {
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

	// Initiate chunked upload
	let (session_id, _) = setup
		.initiate_chunked_upload(&api_token.token, &workspace.id, &repo.name)
		.await;

	// PATCH with 1KB data (buffered in Redis)
	let patch_data: Vec<u8> = (0..=255u8).cycle().take(1024).collect();
	let put_data: Vec<u8> = (128..=255u8).chain(0..=127u8).cycle().take(1024).collect();

	let mut combined = Vec::with_capacity(2048);
	combined.extend_from_slice(&patch_data);
	combined.extend_from_slice(&put_data);
	let digest = sha256_digest(&combined);

	let patch_response = setup
		.patch_blob_chunk(
			&api_token.token,
			&workspace.id,
			&repo.name,
			session_id,
			&patch_data,
		)
		.await;
	assert_eq!(
		patch_response.status_code(),
		StatusCode::ACCEPTED,
		"PATCH chunk failed: {}",
		std::str::from_utf8(&patch_response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// PUT to complete with additional 1KB body + digest of combined data
	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<CompleteBlobUploadPath> {
			path: CompleteBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				session_id,
			},
			query: CompleteBlobUploadQuery {
				digest: digest.clone(),
			},
			headers: CompleteBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: OptionalHeader::new(Some(ContentType::octet_stream())),
				content_length: OptionalHeader::new(Some(ContentLength(put_data.len() as u64))),
				content_range: OptionalHeader::new(None),
			},
			body: Body::from(put_data),
		})
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::CREATED,
		"complete upload failed: {}",
		std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
	);

	// Verify blob GET returns 2KB
	let get_response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: digest.clone(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(get_response.status_code(), StatusCode::OK);
	assert_eq!(get_response.into_bytes().as_ref(), &combined);

	setup
		.assert_blob_size_in_db(&digest, combined.len() as u64)
		.await;
}

/// Pushes a blob larger than the multipart-copy threshold so the finalize
/// step exercises the `UploadPartCopy` fan-out at `registry/blobs/<digest>`
/// instead of the single-shot `CopyObject` branch. Catches regressions in
/// the byte-range math and dest-key wiring.
#[tokio::test]
async fn chunked_upload_exercises_multipart_copy() {
	use api::utils::constants::REGISTRY_BLOB_FINAL_COPY_MULTIPART_THRESHOLD;

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

	let (session_id, _) = setup
		.initiate_chunked_upload(&api_token.token, &workspace.id, &repo.name)
		.await;

	// One byte past the threshold so finalize takes the multipart-copy branch.
	let size = (REGISTRY_BLOB_FINAL_COPY_MULTIPART_THRESHOLD as usize) + 1;
	let data: Vec<u8> = (0..=255u8).cycle().take(size).collect();
	let digest = sha256_digest(&data);

	let patch_response = setup
		.patch_blob_chunk(
			&api_token.token,
			&workspace.id,
			&repo.name,
			session_id,
			&data,
		)
		.await;
	assert_eq!(
		patch_response.status_code(),
		StatusCode::ACCEPTED,
		"PATCH chunk failed: {}",
		std::str::from_utf8(&patch_response.into_bytes()).unwrap_or("<non-utf8>")
	);

	let response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<CompleteBlobUploadPath> {
			path: CompleteBlobUploadPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				session_id,
			},
			query: CompleteBlobUploadQuery {
				digest: digest.clone(),
			},
			headers: CompleteBlobUploadRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				content_type: OptionalHeader::new(None),
				content_length: OptionalHeader::new(None),
				content_range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;
	assert_eq!(
		response.status_code(),
		StatusCode::CREATED,
		"complete upload failed: {}",
		std::str::from_utf8(&response.into_bytes()).unwrap_or("<non-utf8>")
	);

	let get_response = setup
		.make_registry_call(RegistryUnprocessedApiRequest::<GetBlobPath> {
			path: GetBlobPath {
				workspace_id: workspace.id,
				repo_name: repo.name.clone(),
				digest: digest.clone(),
			},
			query: (),
			headers: GetBlobRequestHeaders {
				authorization: BearerToken::from_str(&api_token.token).unwrap(),
				range: OptionalHeader::new(None),
			},
			body: Body::empty(),
		})
		.await;

	assert_eq!(get_response.status_code(), StatusCode::OK);
	let bytes = get_response.into_bytes();
	assert_eq!(bytes.len(), size);
	assert_eq!(bytes.as_ref(), data.as_slice());

	setup.assert_blob_size_in_db(&digest, size as u64).await;
}
