use axum::{http::StatusCode, response::IntoResponse};

use crate::prelude::*;

/// Deletes a blob from the registry.
/// See [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-blobs)
#[axum::debug_handler]
pub(super) async fn handle() -> impl IntoResponse {
	trace!("Delete Blob Called");

	(
		StatusCode::METHOD_NOT_ALLOWED,
		[
			("Content-Type".to_string(), "application/json".to_string()),
			(
				"Docker-Distribution-Api-Version".to_string(),
				"registry/2.0".to_string(),
			),
		],
	)
		.into_response()
}
