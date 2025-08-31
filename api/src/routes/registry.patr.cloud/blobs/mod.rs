use axum::{
	Router,
	routing::{get, head, patch, post, put},
};

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
		.route("/{digest}", get(digest::handle))
		.route("/{digest}", head(digest::handle))
		.route("/upload", post(upload_post::handle))
		.route("/upload/{reference}", put(upload_put::handle))
		.route("/upload/{reference}", patch(upload_patch::handle))
		.with_state(state.clone())
}
