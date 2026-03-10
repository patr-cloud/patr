mod auth;
mod cache;
mod common;
mod loki_push;
mod models;
mod otlp_push;

use std::time::Duration;

use axum::{Router, routing::post};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// TTL for cached runner/deployment lookups: 1 week.
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Maximum body size for incoming log push requests (5 MB).
const MAX_BODY_SIZE: usize = 5 * 1024 * 1024;

/// Sets up the routes for loki.patr.cloud
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr("/loki/api/v1/push", post(loki_push::handle_loki_push))
		.route_with_tsr("/otlp/v1/logs", post(otlp_push::handle_otlp_push))
		.with_state(state.clone())
}
