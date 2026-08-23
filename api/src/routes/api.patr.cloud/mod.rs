mod api_environment;
mod auth;
mod user;
pub(crate) mod workspace;

use axum::Router;

use self::api_environment::*;
use crate::prelude::*;

/// Sets up the routes for the API
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.with_state(state.clone())
		.mount_endpoint(get_api_environment, state, allowed_client_type)
		.merge(auth::setup_routes(state, allowed_client_type).await)
		.merge(user::setup_routes(state, allowed_client_type).await)
		.merge(workspace::setup_routes(state, allowed_client_type).await)
}
