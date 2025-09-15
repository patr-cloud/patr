use axum::{http::StatusCode, response::IntoResponse};

use crate::prelude::*;

/// Handles the `GET /v2/` route.
#[axum::debug_handler]
pub(super) async fn handle() -> impl IntoResponse {
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
