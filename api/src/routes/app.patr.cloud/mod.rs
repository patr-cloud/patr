use axum::{
	body::{Body, BoxBody},
	extract::State,
	http::{Request, Response, StatusCode, Uri},
	response::{IntoResponse, Response as AxumResponse},
	routing::post,
	Router,
};
use leptos::{Errors, LeptosOptions};
use leptos_axum::LeptosRoutes;
use tower::ServiceExt;

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
		.fallback(file_and_error_handler)
		.with_state(config.leptos_options)
		.with_state(state.clone())
}

pub async fn file_and_error_handler(
	uri: Uri,
	State(options): State<LeptosOptions>,
	req: Request<Body>,
) -> AxumResponse {
	let root = options.site_root.clone();
	let res = get_static_file(uri.clone(), &root).await.unwrap();

	if res.status() == StatusCode::OK {
		res.into_response()
	} else {
		let mut errors = Errors::default();
		let handler = leptos_axum::render_app_to_stream(
			options.to_owned(),
			move |cx| leptos::view! {cx, <div>404 not found</div>},
		);
		handler(req).await.into_response()
	}
}

async fn get_static_file(uri: Uri, root: &str) -> Result<Response<BoxBody>, (StatusCode, String)> {
	let req = Request::builder()
		.uri(uri.clone())
		.body(Body::empty())
		.unwrap();
	// `ServeDir` implements `tower::Service` so we can call it with
	// `tower::ServiceExt::oneshot` This path is relative to the cargo root
	match tower_http::services::ServeDir::new(root).oneshot(req).await {
		Ok(res) => Ok(res.map(axum::body::boxed)),
		Err(err) => Err((
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Something went wrong: {err}"),
		)),
	}
}
