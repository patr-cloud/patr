use axum::Router;

use crate::prelude::*;

/// All deployment related handlers
mod deployment;
/// Get the information of a workspace
mod get_workspace_info;

use self::get_workspace_info::*;

#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Send + 'static,
{
	Router::new()
		.merge(deployment::setup_routes(state).await)
		.mount_auth_endpoint(get_workspace_info, state)
}
