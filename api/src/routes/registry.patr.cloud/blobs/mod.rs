use axum::{
	Router,
	routing::{get, post},
};

use crate::prelude::*;

/// Get and Head routes for blob digest
mod digest;
/// POST request for  blob upload
mod uploads;

pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/:digest", get(digest::handle).head(digest::handle))
		.route("/upload", post(uploads::handle))
		.with_state(state.clone())
}
