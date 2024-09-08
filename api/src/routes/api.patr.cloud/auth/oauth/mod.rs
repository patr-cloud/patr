use axum::Router;

use crate::prelude::*;

/// Sets up the oauth routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
}
