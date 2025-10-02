use axum::{Router, routing::put};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// Delete Manifest route
mod delete;
/// Push Manifest route
mod put_reference;
/// Head Manifest route
mod reference;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr(
			"/{reference}",
			put(put_reference::handle)
				.get(reference::handle)
				.head(reference::handle)
				.delete(delete::handle),
		)
		.with_state(state.clone())
}
