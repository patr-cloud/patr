use axum::{Router, routing::get};

use crate::prelude::*;

/// List All Tags
mod list;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/{list}", get(list::handle))
		.with_state(state.clone())
}
