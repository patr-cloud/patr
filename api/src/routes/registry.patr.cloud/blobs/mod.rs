use axum::Router;

/// Get and Head routes for digest
mod digest;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new().route(
		"/:digest",
		get(get_blob_info::handle).head(get_blob_info::handle),
	)
}
