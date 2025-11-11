//! Tags listing handlers.
//!
//! This module contains handlers for listing tags in a repository.

use axum::Router;

use crate::routes::registry_patr_cloud::prelude::*;

mod list;

pub use self::list::*;

/// Setup the tags routes.
pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		// ============================================================
		// 3. Tags Operations (Authenticated)
		// ============================================================
		// GET /v2/{name}/tags/list - List all tags in a repository
		// Note: Must be mounted before manifest operations to avoid conflicts
		.mount_auth_registry_endpoint(list_tags, state)
}
