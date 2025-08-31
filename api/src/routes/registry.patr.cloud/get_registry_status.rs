use axum::{http::StatusCode, response::IntoResponse};

use crate::prelude::*;

/// Handles the `GET /v2/` route.
#[axum::debug_handler]
pub(super) async fn handle() -> impl IntoResponse {
	trace!("Registry status check");
	StatusCode::OK
}
