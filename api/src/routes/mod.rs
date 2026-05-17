use std::any::Any;

use axum::{Router, response::IntoResponse};
use models::ApiErrorResponse;
use tower_http::catch_panic::CatchPanicLayer;

use crate::prelude::*;

/// The routes for serving <https://api.patr.cloud>
#[path = "api.patr.cloud/mod.rs"]
pub mod api_patr_cloud;

/// The routes for serving the backend on <https://app.patr.cloud/api>. In
/// self-hosted builds, only the `proxy` fn is exposed (reused by the
/// single-domain router as its fallback); `setup_routes` is cloud-only.
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

/// Turns a panic caught while handling a request into a 500 response instead of
/// letting it tear down the connection. Shared by both the cloud and
/// self-hosted routers below.
fn on_request_panic(panic: Box<dyn Any + Send>) -> axum::response::Response {
	let details = panic
		.downcast_ref::<&str>()
		.map(|message| (*message).to_owned())
		.or_else(|| panic.downcast_ref::<String>().cloned())
		.unwrap_or_else(|| "unknown panic".to_owned());
	error!("caught panic while handling request: {details}");
	ApiErrorResponse::error(ErrorType::InternalServerError).into_response()
}

cfg_if! {
	if #[cfg(feature = "cloud")] {
		use axum::{
			body::Body,
			http::{Request, Response, StatusCode},
			routing::any,
		};
		use headers::{HeaderMapExt, Host};
		use tower::ServiceExt;

		/// Sets up the routes for the API. In cloud mode, fans out by `Host`
		/// header to the six platform subdomains.
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
				.layer(CatchPanicLayer::custom(on_request_panic))
				.with_state(state.clone())
		}
	} else {
		/// Self-hosted router: single base domain, path-prefix fanout.
		/// Registry takes `/v2` (OCI requirement). Everything else lives
		/// behind named prefixes; the fallback proxies to the frontend.
		#[instrument(skip(state))]
		pub async fn setup_routes(state: &AppState) -> Router {
			Router::new()
				.merge(registry_patr_cloud::setup_routes(state).await)
				.merge(loki_patr_cloud::setup_routes(state).await)
				.nest(
					"/api",
					api_patr_cloud::setup_routes(state, ClientType::WebDashboard).await,
				)
				.nest("/mimir", mimir_patr_cloud::setup_routes(state).await)
				.nest("/assets", assets_patr_cloud::setup_routes(state).await)
				.fallback(app_patr_cloud::proxy)
				.layer(CatchPanicLayer::custom(on_request_panic))
		}
	}
}
