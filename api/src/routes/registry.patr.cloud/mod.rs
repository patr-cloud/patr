//! OCI-compliant Docker registry implementation.
//!
//! This module implements the OCI Distribution Specification v1.0+ for the Patr
//! platform. All registry-specific code is contained within this directory to
//! maintain clear separation.
//!
//! ## Architecture
//!
//! - `endpoint.rs`: RegistryEndpoint trait definition
//! - `request.rs`: RegistryRequest and AuthenticatedRegistryRequest types
//! - `error.rs`: Registry-specific error types
//! - `response.rs`: RegistryResponse type with streaming support
//! - `handlers/`: OCI Distribution API endpoint implementations
//! - `utils/`: S3 streaming, repository validation, and other utilities

mod endpoint;
mod request;
mod response;

/// Registry-specific error types.
mod error;
/// OCI Distribution API endpoint handlers.
mod handlers;
/// Utility functions for registry operations.
mod utils;

use std::convert::Infallible;

use axum::{Router, body::Body, response::IntoResponse};
use http::{Request, StatusCode};

use crate::prelude::*;

/// A prelude module for easy importing of common types specific to the registry
/// routes.
pub mod prelude {
	pub use axum::body::Body;
	pub use http::{StatusCode, status};
	pub use models::prelude::*;
	pub use oci_spec::distribution::*;

	pub use crate::{
		prelude::*,
		routes::registry_patr_cloud::{
			endpoint::RegistryEndpoint,
			error::RegistryError,
			request::{
				AuthenticatedRegistryAppRequest,
				RegistryAppRequest,
				RegistryProcessedApiRequest,
				RegistryUnprocessedApiRequest,
				RegistryUnprocessedAppRequest,
			},
			response::RegistryResponse,
			utils::{BodyStreamWrapper, ReadBufferedBytesExt, S3UploadSession},
		},
		utils::RouterExt,
	};
}

/// Setup registry routes.
///
/// This function mounts all OCI Distribution API endpoints according to the
/// OCI Distribution Specification v1.0+. Endpoints are mounted in a specific
/// order to avoid path conflicts:
/// 1. Version check (no auth required)
/// 3. Tags listing (special path: /tags/list)
/// 4. Blob upload operations (specific paths with /uploads/)
/// 5. Manifest operations (generic path with {reference})
/// 6. Blob operations (generic path with {digest})
///
/// ## Path Conflict Prevention
/// The order is important to prevent path conflicts:
/// - `/v2/{workspace_id}/{name}/tags/list` must be mounted before
///   `/v2/{workspace_id}/{name}/manifests/{reference}`
/// - `/v2/{workspace_id}/{name}/blobs/uploads/` must be mounted before
///   `/v2/{workspace_id}/{name}/blobs/{digest}`
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	handlers::setup_routes(state)
		.await
		.fallback(async |req: Request<Body>| {
			warn!("Unhandled registry request: {} {}", req.method(), req.uri());
			Ok::<_, Infallible>((StatusCode::NOT_FOUND, "Not Found").into_response())
		})
}
