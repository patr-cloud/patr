mod workspace;

use axum::Router;

use crate::prelude::*;

/// Sets up the routes for the API, across all domains.
#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Clone + 'static,
{
	Router::new().merge(workspace::setup_routes(state).await)
}
