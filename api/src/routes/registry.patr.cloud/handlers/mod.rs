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

/// Blob handlers for the OCI registry.
pub mod blob;
/// Manifest handlers for the OCI registry.
pub mod manifest;
/// Version check handler for the OCI registry.
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
/// - `/v2/{workspace_id}/{repo_name}/tags/list` must be mounted before
///   `/v2/{workspace_id}/{repo_name}/manifests/{reference}`
/// - `/v2/{workspace_id}/{repo_name}/blobs/uploads/` must be mounted before
///   `/v2/{workspace_id}/{repo_name}/blobs/{digest}`
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_registry_endpoint(version_check, state)
		.merge(manifest::setup_routes(state).await)
		.merge(blob::setup_routes(state).await)
}
