use axum::{
	body::Body,
	extract::Host,
	http::{Request, Response, StatusCode},
	routing::any,
	Router,
};
use tower::ServiceExt;

use crate::prelude::*;

/// The routes for serving https://api.patr.cloud
#[path = "api.patr.cloud/mod.rs"]
#[allow(dead_code, missing_docs, clippy::missing_docs_in_private_items)]
mod api_patr_cloud;

/// The routes for serving https://app.patr.cloud
#[path = "app.patr.cloud/mod.rs"]
mod app_patr_cloud;

/// The routes for serving https://registry.patr.cloud as a docker registry
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
			api_router.oneshot(request).await
		} else {
			match hostname.as_str() {
				"api.patr.cloud" => api_router.oneshot(request).await,
				"registry.patr.cloud" => registry_router.oneshot(request).await,
				"app.patr.cloud" => app_router.oneshot(request).await,
				_ => Ok(Response::builder()
					.status(StatusCode::NOT_FOUND)
					.body(Body::empty())
					.unwrap()),
			}
		}
	});

	Router::new()
		.fallback(route_handler)
		.with_state(state.clone())
}
