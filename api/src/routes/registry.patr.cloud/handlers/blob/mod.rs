//! Blob endpoint handlers. This module contains handlers for blob operations in
//! the OCI Distribution

use axum::Router;

use crate::routes::registry_patr_cloud::prelude::*;

/// Cancel blob upload
mod cancel_upload;
/// Complete blob upload
mod complete_upload;
/// Delete a blob
mod delete_blob;
/// Download a blob
mod get_blob;
/// Get upload status
mod get_upload_status;
/// Check if a blob exists and get metadata
mod head_blob;
/// Initiate blob upload
mod initiate_upload;
/// Upload blob chunk
mod upload_chunk;

pub use self::{
	cancel_upload::*,
	complete_upload::*,
	delete_blob::*,
	get_blob::*,
	get_upload_status::*,
	head_blob::*,
	initiate_upload::*,
	upload_chunk::*,
};

/// Setup the tags routes.
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		// POST /v2/{workspace_id}/{repo_name}/blobs/uploads/ - Initiate blob upload or mount blob
		// Note: Handles both new uploads and cross-repository blob mounting
		.mount_registry_endpoint(initiate_upload, state)
		// GET /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id} - Get upload status
		.mount_registry_endpoint(get_upload_status, state)
		// PATCH /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id} - Upload blob chunk
		.mount_registry_endpoint(upload_chunk, state)
		// PUT /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id} - Complete blob upload
		.mount_registry_endpoint(complete_upload, state)
		// DELETE /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id} - Cancel blob upload
		.mount_registry_endpoint(cancel_upload, state)
		// HEAD /v2/{workspace_id}/{repo_name}/blobs/{reference} - Check blob existence
		.mount_registry_endpoint(head_blob, state)
		// GET /v2/{workspace_id}/{repo_name}/blobs/{reference} - Download blob
		.mount_registry_endpoint(get_blob, state)
		// DELETE /v2/{workspace_id}/{repo_name}/blobs/{reference} - Delete blob
		.mount_registry_endpoint(delete_blob, state)
}
