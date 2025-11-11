//! Manifest handlers for the OCI registry.
//!
//! This module contains handlers for manifest operations:
//! - GET: Retrieve a manifest by tag or digest
//! - HEAD: Check if a manifest exists and get metadata
//! - PUT: Upload a new manifest
//! - DELETE: Delete a manifest

use axum::Router;

use crate::routes::registry_patr_cloud::prelude::*;

// mod delete;
mod get;
mod head;
mod put;

pub use self::{get::*, head::*, put::*};

/// Setup the tags routes.
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		// ============================================================
		// 5. Manifest Operations (Authenticated)
		// ============================================================
		// GET /v2/{name}/manifests/{reference} - Get manifest by tag or digest
		.mount_auth_registry_endpoint(get_manifest, state)
		// HEAD /v2/{name}/manifests/{reference} - Check manifest existence
		.mount_auth_registry_endpoint(check_manifest, state)
		// PUT /v2/{name}/manifests/{reference} - Upload manifest
		.mount_auth_registry_endpoint(upload_manifest, state)
	// DELETE /v2/{name}/manifests/{reference} - Delete manifest
	// .mount_auth_registry_endpoint(delete_manifest, state)
}
