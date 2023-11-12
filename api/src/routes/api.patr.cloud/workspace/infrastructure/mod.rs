use axum::Router;

use crate::prelude::*;

mod database;
mod deployment;
mod managed_url;
mod static_site;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.merge(database::setup_routes(state).await)
		.merge(deployment::setup_routes(state).await)
		.merge(managed_url::setup_routes(state).await)
		.merge(static_site::setup_routes(state).await)
}
