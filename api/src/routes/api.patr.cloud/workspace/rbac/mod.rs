use axum::Router;

use crate::prelude::*;

mod permission;
mod role;
mod user;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.merge(permission::setup_routes(state, host_client_types).await)
		.merge(role::setup_routes(state, host_client_types).await)
		.merge(user::setup_routes(state, host_client_types).await)
}
