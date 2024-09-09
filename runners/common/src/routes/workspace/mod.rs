use axum::Router;

use crate::prelude::*;

mod deployment;

#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor,
{
	Router::new().merge(deployment::setup_routes(state).await)
}
