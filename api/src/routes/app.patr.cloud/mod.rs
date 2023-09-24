use axum::{routing::post, Router};
use leptos_axum::LeptosRoutes;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/api/*fn_name", post(leptos_axum::handle_server_fns))
		.leptos_routes(
			&leptos::get_configuration(None)
				.await
				.expect("failed to get configuration")
				.leptos_options,
			leptos_axum::generate_route_list(app_fn),
			app_fn,
		)
		.with_state(state.clone())
}
