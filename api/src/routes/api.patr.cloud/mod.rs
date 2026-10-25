mod api_environment;
mod auth;
mod get_version;
mod user;
mod workspace;

use axum::Router;

use self::{api_environment::*, get_version::*};
use crate::prelude::*;

/// Sets up the routes for the API
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.with_state(state.clone())
		.mount_endpoint(get_api_environment, state, host_client_types)
		.merge(auth::setup_routes(state, host_client_types).await)
		.merge(user::setup_routes(state, host_client_types).await)
		.merge(workspace::setup_routes(state, host_client_types).await)
		.mount_endpoint(get_version, state, host_client_types)
}
