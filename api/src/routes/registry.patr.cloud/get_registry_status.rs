use axum::{
	http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
	response::IntoResponse,
};

/// Handles the `GET /v2/` route.
#[axum::debug_handler]
pub(super) async fn handle() -> impl IntoResponse {
	(StatusCode::OK, [].into_iter().collect::<HeaderMap>()).into_response()
}
