/// Authentication helpers for runner-based Basic Auth.
pub mod auth;
/// Redis-backed caching for runner ownership lookups.
pub mod cache;
/// Upstream forwarding to OpenBao.
pub mod common;
/// Handler for OpenBao KV v2 reads
/// (`/v1/secret/data/{workspace_id}/{secret_id}`).
pub mod read_secret;

use std::time::Duration;

use axum::{Router, routing::get};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// TTL for cached runner lookups: 1 week.
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Sets up the routes for openbao.patr.cloud
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr(
			"/v1/secret/data/{workspace_id}/{secret_id}",
			get(read_secret::handle_read_secret),
		)
		.with_state(state.clone())
}
