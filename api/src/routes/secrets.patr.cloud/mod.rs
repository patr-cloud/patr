/// Authentication helpers for runner-based Basic Auth.
pub mod auth;
/// Upstream forwarding to OpenBao.
pub mod common;
/// Handler for OpenBao KV v2 reads
/// (`/v1/secret/data/{workspace_id}/{secret_id}`).
pub mod read_secret;

use axum::{Router, routing::get};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// Sets up the routes for secrets.patr.cloud
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr(
			"/v1/secret/data/{workspace_id}/{secret_id}",
			get(read_secret::handle_read_secret),
		)
		.with_state(state.clone())
}
