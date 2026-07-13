//! Tags handlers for the OCI registry.

use axum::Router;

use crate::prelude::*;

/// Tags-list endpoint (a 405 stub — listing is via the Patr API).
mod list;

pub use self::list::*;

/// Setup tags routes.
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new().mount_registry_endpoint(list_tags, state)
}
