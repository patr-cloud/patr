use axum::Router;

use crate::prelude::*;

mod permission;
mod role;
mod user;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.merge(permission::setup_routes(state, allowed_client_type).await)
		.merge(role::setup_routes(state, allowed_client_type).await)
		.merge(user::setup_routes(state, allowed_client_type).await)
}
