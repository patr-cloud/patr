use axum::{
	Router,
	routing::{get, head, put},
};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// Push Manifest route
mod put;
/// Head Manifest route
mod reference;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr("/{reference}", put(put::handle))
		.route_with_tsr("/{reference}", get(reference::handle))
		.route_with_tsr("/{reference}", head(reference::handle))
		.with_state(state.clone())
}
