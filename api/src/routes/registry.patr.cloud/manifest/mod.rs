use axum::{Router, routing::put};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// Push Manifest route
mod push;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr("/{reference}", put(push::handle))
		.with_state(state.clone())
}
