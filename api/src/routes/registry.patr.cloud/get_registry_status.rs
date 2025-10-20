use axum::{http::StatusCode, response::IntoResponse};

use crate::prelude::*;

macros::declare_registry_endpoint!(
	/// The route to get the manifest information of a specific image.
	///
	/// Declaration: https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-manifests
	GetRegistryStatus,
	GET "/v2/",
);

/// Handles the `GET /v2/` route.
#[axum::debug_handler]
pub(super) async fn get_registry_status() -> impl IntoResponse {
	trace!("Registry status check");
	(
		StatusCode::OK,
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
