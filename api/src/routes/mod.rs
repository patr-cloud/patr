use std::any::Any;

use axum::{
	Router,
	body::Body,
	http::{Request, Response, StatusCode},
	response::IntoResponse,
	routing::any,
};
use headers::{HeaderMapExt, Host};
use models::ApiErrorResponse;
use tower::ServiceExt;
use tower_http::catch_panic::CatchPanicLayer;

use crate::prelude::*;

/// The routes for serving <https://api.patr.cloud>
#[path = "api.patr.cloud/mod.rs"]
pub mod api_patr_cloud;

/// The routes for serving the backend on <https://app.patr.cloud/api>
#[path = "app.patr.cloud/mod.rs"]
pub mod app_patr_cloud;

/// The routes for serving https://assets.patr.cloud for static assets
#[path = "assets.patr.cloud/mod.rs"]
pub mod assets_patr_cloud;

/// The routes for serving https://loki.patr.cloud as an authenticated Loki
/// push proxy
#[path = "loki.patr.cloud/mod.rs"]
pub mod loki_patr_cloud;

/// The routes for serving https://mimir.patr.cloud as an authenticated Mimir
/// push proxy
#[path = "mimir.patr.cloud/mod.rs"]
pub mod mimir_patr_cloud;

/// The routes for serving https://registry.patr.cloud as a docker registry
#[path = "registry.patr.cloud/mod.rs"]
pub mod registry_patr_cloud;

/// Sets up the routes for the API, across all domains.
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	let api_router = api_patr_cloud::setup_routes(state, ClientType::ApiToken).await;
	let app_router = app_patr_cloud::setup_routes(state).await;
	let assets_router = assets_patr_cloud::setup_routes(state).await;
	let loki_router = loki_patr_cloud::setup_routes(state).await;
	let mimir_router = mimir_patr_cloud::setup_routes(state).await;
	let registry_router = registry_patr_cloud::setup_routes(state).await;

	Router::new()
		.fallback(any(async |request: Request<Body>| {
			let hostname = request.headers().typed_get::<Host>();
			let hostname = hostname
				.as_ref()
				.map(|host| host.hostname())
				.unwrap_or_default();
			match hostname {
				"api.patr.cloud" => api_router.oneshot(request).await,
				"app.patr.cloud" => app_router.oneshot(request).await,
				"assets.patr.cloud" => assets_router.oneshot(request).await,
				"loki.patr.cloud" => loki_router.oneshot(request).await,
				"mimir.patr.cloud" => mimir_router.oneshot(request).await,
				"registry.patr.cloud" => registry_router.oneshot(request).await,
				_ => Ok(Response::builder()
					.status(StatusCode::NOT_FOUND)
					.body(Body::empty())
					.unwrap()),
			}
		}))
		.layer(CatchPanicLayer::custom(|panic: Box<dyn Any + Send>| {
			let details = panic
				.downcast_ref::<&str>()
				.map(|message| (*message).to_owned())
				.or_else(|| panic.downcast_ref::<String>().cloned())
				.unwrap_or_else(|| "unknown panic".to_owned());
			error!("caught panic while handling request: {details}");
			ApiErrorResponse::error(ErrorType::InternalServerError).into_response()
		}))
		.with_state(state.clone())
}
