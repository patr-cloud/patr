use axum::{
	body::{self, Body},
	extract::Host,
	http::{Request, Response, StatusCode},
	routing::any,
	Router,
};
use tower::ServiceExt;

use crate::prelude::*;

#[path = "api.patr.cloud/mod.rs"]
mod api_patr_cloud;

#[path = "registry.patr.cloud/mod.rs"]
mod registry_patr_cloud;

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	let api_router = api_patr_cloud::setup_routes(state);
	let registry_router = registry_patr_cloud::setup_routes(state);

	Router::new()
		.route(
			"/",
			any(|Host(hostname), request: Request<Body>| async move {
				match hostname.as_str() {
					"api.patr.cloud" => api_router.oneshot(request).await,
					"registry.patr.cloud" => registry_router.oneshot(request).await,
					_ => Ok(Response::builder()
						.status(StatusCode::NOT_FOUND)
						.body(body::boxed(Body::empty()))
						.unwrap()),
				}
			}),
		)
		.with_state(state.clone())
}
