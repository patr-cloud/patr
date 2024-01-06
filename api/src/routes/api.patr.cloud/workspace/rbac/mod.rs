use axum::Router;

use crate::prelude::*;

mod permission;
mod role;
mod user;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.merge(permission::setup_routes(state).await)
		.merge(role::setup_routes(state).await)
		.merge(user::setup_routes(state).await)
}
