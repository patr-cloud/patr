use axum::{
	http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
	response::IntoResponse,
};

use crate::utils::layers::RegistryRequest;

/// Handles the `GET /v2/` route.
#[axum::debug_handler]
pub(super) async fn handle() -> impl IntoResponse {
	(
		[(
			HeaderName::from_static("Docker-Distribution-API-Version"),
			HeaderValue::from_static("registry/2.0"),
		)]
		.into_iter()
		.collect::<HeaderMap>(),
		StatusCode::OK,
	)
}
