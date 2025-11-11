//! Registry endpoint handlers.
//!
//! This module contains all the OCI Distribution API endpoint implementations:
//! - Version check (GET /v2/)
//! - Manifest operations (GET, PUT, DELETE, HEAD)
//! - Blob operations (GET, HEAD, DELETE, upload)
//! - Tags listing
//!
//! Handlers will be implemented in tasks 10-26.

use axum::Router;

use crate::prelude::*;

// mod blob;
mod manifest;
// mod tags;
mod version_check;

pub use self::version_check::*;

/// Setup registry routes.
///
/// This function mounts all OCI Distribution API endpoints according to the
/// OCI Distribution Specification v1.0+. Endpoints are mounted in a specific
/// order to avoid path conflicts:
///
/// 1. Version check (no auth required)
/// 3. Tags listing (special path: /tags/list)
/// 4. Blob upload operations (specific paths with /uploads/)
/// 5. Manifest operations (generic path with {reference})
/// 6. Blob operations (generic path with {digest})
///
/// ## Path Conflict Prevention
///
/// The order is important to prevent path conflicts:
/// - `/v2/{name}/tags/list` must be mounted before
///   `/v2/{name}/manifests/{reference}`
/// - `/v2/{name}/blobs/uploads/` must be mounted before
///   `/v2/{name}/blobs/{digest}`
///
/// ## Requirements
///
/// - 1.4: Use RouterExt trait for consistency
/// - 1.6: Organize code with separate handler modules
/// - 12.1: All handlers receive database transaction
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_registry_endpoint(version_check, state)
		.merge(manifest::setup_routes(state).await)
	// // ============================================================
	// // 4. Blob Upload Operations (Authenticated)
	// // ============================================================
	// // POST /v2/{name}/blobs/uploads/ - Initiate blob upload or mount blob
	// // Note: Handles both new uploads and cross-repository blob mounting
	// .mount_auth_registry_endpoint(blob::initiate_upload::handler, state)
	// // GET /v2/{name}/blobs/uploads/{uuid} - Get upload status
	// .mount_auth_registry_endpoint(blob::get_upload_status::handler, state)
	// // PATCH /v2/{name}/blobs/uploads/{uuid} - Upload blob chunk
	// .mount_auth_registry_endpoint(blob::upload_chunk::handler, state)
	// // PUT /v2/{name}/blobs/uploads/{uuid} - Complete blob upload
	// .mount_auth_registry_endpoint(blob::complete_upload::handler, state)
	// // DELETE /v2/{name}/blobs/uploads/{uuid} - Cancel blob upload
	// .mount_auth_registry_endpoint(blob::cancel_upload::handler, state)
	// // ============================================================
	// // 6. Blob Operations (Authenticated)
	// // ============================================================
	// // HEAD /v2/{name}/blobs/{digest} - Check blob existence
	// .mount_auth_registry_endpoint(blob::head::handler, state)
	// // GET /v2/{name}/blobs/{digest} - Download blob
	// .mount_auth_registry_endpoint(blob::get::handler, state)
	// // DELETE /v2/{name}/blobs/{digest} - Delete blob
	// .mount_auth_registry_endpoint(blob::delete::handler, state)
}
