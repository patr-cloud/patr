use axum::{
	Router,
	routing::{get, post, put},
};
use axum_extra::routing::RouterExt;

use crate::prelude::*;

/// Get and Head routes for blob digest
mod digest;
/// PATCH request for blob upload
mod upload_patch;
/// POST request for  blob upload
mod upload_post;
/// PUT request for blob upload
mod upload_put;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route_with_tsr("/{digest}", get(digest::handle).head(digest::handle))
		.route_with_tsr("/upload", post(upload_post::handle))
		.route_with_tsr(
			"/upload/{reference}",
			put(upload_put::handle).patch(upload_patch::handle),
		)
		.with_state(state.clone())
}
