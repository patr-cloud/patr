mod auth;
mod user;
mod workspace;

use axum::Router;

use crate::prelude::*;

/// Sets up the routes for the API
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.with_state(state.clone())
		.merge(auth::setup_routes(state, allowed_client_types).await)
		.merge(user::setup_routes(state, allowed_client_types).await)
		.merge(workspace::setup_routes(state, allowed_client_types).await)
}
