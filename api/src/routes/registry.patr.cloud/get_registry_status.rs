use axum::{body::Body, http::StatusCode};
use headers::ContentType;

use super::types::{RegistryResponse, RegistryError};
use crate::prelude::*;

macros::declare_registry_endpoint!(
	/// The endpoint for checking the registry status
	GetRegistryStatus,
	GET "/v2",
	response_headers = {
		/// Content-Type header
		pub content_type: ContentType,
	}
);

/// Handles the `GET /v2/` route.
#[axum::debug_handler]
pub(super) async fn get_registry_status()
-> Result<RegistryResponse<GetRegistryStatusRequest>, RegistryError> {
	trace!("Registry status check");
	Ok(RegistryResponse {
		status: StatusCode::OK,
		headers: GetRegistryStatusResponseHeaders {
			content_type: ContentType::json(),
			// "Docker-Distribution-Api-Version": "registry/2.0".to_string(),
		},
		body: Body::empty(),
	})
}
