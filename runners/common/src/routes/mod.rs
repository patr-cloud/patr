mod workspace;

use axum::Router;

use crate::prelude::*;

/// Sets up the routes for the API, across all domains.
#[instrument(skip(_state))]
pub async fn setup_routes<E>(_state: &AppState<E>) -> Router
where
	E: RunnerExecutor,
{
	Router::new()
}
