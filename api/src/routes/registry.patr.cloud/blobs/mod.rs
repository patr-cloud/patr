use axum::{
	Router,
	routing::{get, post, put},
};

use crate::prelude::*;

/// Get and Head routes for blob digest
mod digest;
/// POST request for  blob upload
mod upload_post;
/// PUT request for blob upload
mod upload_put;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/:digest", get(digest::handle).head(digest::handle))
		.route("/upload", post(upload_post::handle))
		.route("/upload/:reference", put(upload_put::handle))
		.with_state(state.clone())
}
