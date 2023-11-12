use axum::{http::StatusCode, Router};
use models::{ApiRequest, ErrorType};

use crate::prelude::*;

mod database;
mod deployment;
mod managed_url;
mod static_site;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.merge(database::setup_routes(state))
		.merge(deployment::setup_routes(state))
		.merge(managed_url::setup_routes(state))
		.merge(static_site::setup_routes(state))
}
