use axum::{Router, routing::get};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// List All Tags
mod list;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr("/{list}", get(list::handle))
		.with_state(state.clone())
}
