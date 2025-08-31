use axum::{Router, routing::put};

use crate::prelude::*;

/// Push Manifest route
mod push;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/{reference}", put(push::handle))
		.with_state(state.clone())
}
