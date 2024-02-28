mod auth;
mod user;

#[allow(unused_variables, dead_code, unreachable_code, unused_imports)]
mod workspace;

use axum::Router;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.merge(auth::setup_routes(state).await)
		.merge(user::setup_routes(state).await)
		.merge(workspace::setup_routes(state).await)
}
