use axum::Router;

use crate::prelude::*;

mod infrastructure;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new().merge(infrastructure::setup_routes(state).await)
}
