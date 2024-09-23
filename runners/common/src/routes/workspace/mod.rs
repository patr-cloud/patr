use axum::Router;

use crate::prelude::*;

/// All deployment related handlers
pub mod deployment;

#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Clone + 'static,
{
	Router::new().merge(deployment::setup_routes(state).await)
}
