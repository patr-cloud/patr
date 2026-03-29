/// Authentication helpers for runner-based Basic Auth.
pub mod auth;
/// Redis-backed caching for runner/deployment ownership lookups.
pub mod cache;
/// Shared label/attribute validation, rewriting, and upstream forwarding.
pub mod common;
/// Prometheus remote write protobuf model types.
pub mod models;
/// Handler for OTLP metrics push requests (`/otlp/v1/metrics`).
pub mod otlp_metrics_push;
/// Handler for Prometheus remote write push requests (`/api/v1/push`).
pub mod remote_write_push;

use std::time::Duration;

use axum::{Router, routing::post};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// TTL for cached runner/deployment lookups: 1 week.
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Maximum body size for incoming metrics push requests (5 MB).
const MAX_BODY_SIZE: usize = 5 * 1024 * 1024;

/// Sets up the routes for mimir.patr.cloud
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr(
			"/api/v1/push",
			post(remote_write_push::handle_remote_write_push),
		)
		.route_with_tsr(
			"/otlp/v1/metrics",
			post(otlp_metrics_push::handle_otlp_metrics_push),
		)
		.with_state(state.clone())
}
