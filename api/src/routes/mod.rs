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
#[path = "app.patr.cloud/mod.rs"]
mod app_patr_cloud;
#[path = "registry.patr.cloud/mod.rs"]
mod registry_patr_cloud;

/// Sets up the routes for the API, across all domains.
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	let api_router = api_patr_cloud::setup_routes(state).await;
	let app_router = app_patr_cloud::setup_routes(state).await;
	let registry_router = registry_patr_cloud::setup_routes(state).await;

	let route_handler = any(|Host(hostname), request: Request<Body>| async move {
		if cfg!(debug_assertions) {
			app_router.oneshot(request).await
		} else {
			match hostname.as_str() {
				"api.patr.cloud" => api_router.oneshot(request).await,
				"registry.patr.cloud" => registry_router.oneshot(request).await,
				"app.patr.cloud" => app_router.oneshot(request).await,
				_ => Ok(Response::builder()
					.status(StatusCode::NOT_FOUND)
					.body(body::boxed(Body::empty()))
					.unwrap()),
			}
		}
	});

	Router::new()
		.fallback(route_handler)
		.with_state(state.clone())
}
