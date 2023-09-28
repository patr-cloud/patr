use std::path::Path;

use axum::{
	body::Body,
	extract::State,
	http::{Request, StatusCode},
	response::{IntoResponse, Response as AxumResponse},
	routing::{any, post},
	Router,
};
use leptos::LeptosOptions;
use leptos_axum::LeptosRoutes;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	let config = leptos::get_configuration(None)
		.await
		.expect("failed to get configuration");
	Router::new()
		.route(
			"/api/*fn_name",
			post(leptos_axum::handle_server_fns).get(leptos_axum::handle_server_fns),
		)
		.leptos_routes(
			&config.leptos_options,
			leptos_axum::generate_route_list(frontend::render).await,
			frontend::render,
		)
		.fallback(serve_file)
		.with_state(config.leptos_options)
		.with_state(state.clone())
}

async fn serve_file(State(options): State<LeptosOptions>, req: Request<Body>) -> AxumResponse {
	println!("Falling back to serving file: {}", req.uri());
	let response = ServeDir::new(Path::new(options.site_root.as_str()))
		.oneshot(
			Request::builder()
				.uri(req.uri().clone())
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.map_err(|err| match err {})
		.into_response();

	if response.status() != StatusCode::OK {
		println!("File not found: {}", req.uri());
		(leptos_axum::render_app_to_stream(options, frontend::pages::NotFound))(req)
			.await
			.into_response()
	} else {
		println!("File found: {}", req.uri());
		response
	}
}
