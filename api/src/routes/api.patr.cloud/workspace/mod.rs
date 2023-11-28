use axum::Router;
use crate::prelude::*;

mod infrastructure;
mod domain;
mod region;
mod secret;
mod rbac;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
	.merge(infrastructure::setup_routes(state).await)
	.merge(domain::setup_routes(state).await)
	.merge(region::setup_routes(state).await)
	.merge(secret::setup_routes(state).await)
	.merge(rbac::setup_routes(state).await)
}
