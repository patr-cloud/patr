//! Manifest handlers for the OCI registry.

use axum::Router;

use crate::routes::registry_patr_cloud::prelude::*;

/// Delete a manifest
mod delete;
/// Retrieve a manifest by tag or digest
mod get;
/// Check if a manifest exists and get metadata
mod head;
/// Upload a new manifest
mod put;

pub use self::{delete::*, get::*, head::*, put::*};

/// Setup the tags routes.
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		// GET /v2/{name}/manifests/{reference} - Get manifest by tag or digest
		.mount_registry_endpoint(get_manifest, state)
		// HEAD /v2/{name}/manifests/{reference} - Check manifest existence
		.mount_registry_endpoint(check_manifest, state)
		// PUT /v2/{name}/manifests/{reference} - Upload manifest
		.mount_registry_endpoint(upload_manifest, state)
		// DELETE /v2/{name}/manifests/{reference} - Delete manifest
		.mount_registry_endpoint(delete_manifest, state)
}
