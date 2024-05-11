use axum::Router;

use crate::prelude::*;

// mod container_registry;
mod database;
mod deployment;
// mod domain;
mod managed_url;
mod rbac;
mod runner;
mod secret;
// mod static_site;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		// .merge(container_registry::setup_routes(state).await)
		.merge(database::setup_routes(state).await)
		.merge(deployment::setup_routes(state).await)
		// .merge(domain::setup_routes(state).await)
		.merge(managed_url::setup_routes(state).await)
		.merge(rbac::setup_routes(state).await)
		.merge(runner::setup_routes(state).await)
		.merge(secret::setup_routes(state).await)
	// .merge(static_site::setup_routes(state).await)
}
