mod auth;

use axum::Router;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new().merge(auth::setup_routes(state))
}
