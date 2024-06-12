mod auth;
mod user;
mod workspace;

use axum::Router;

use crate::prelude::*;

/// Sets up the routes for the API
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.with_state(state.clone())
		.merge(auth::setup_routes(state).await)
		.merge(user::setup_routes(state).await)
		.merge(workspace::setup_routes(state).await)
}
