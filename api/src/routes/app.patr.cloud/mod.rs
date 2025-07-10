use axum::Router;
use dioxus::prelude::*;

use crate::prelude::*;

/// Sets up the routes for the web dashboard
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.serve_dioxus_application(ServeConfigBuilder::new(), frontend::app::App)
		.with_state(state.clone())
}
