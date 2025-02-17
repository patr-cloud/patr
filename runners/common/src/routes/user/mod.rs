mod get_user_info;
mod list_workspaces;

use axum::Router;

use self::{get_user_info::*, list_workspaces::*};
use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Send + 'static,
{
	Router::new()
		.mount_auth_endpoint(list_workspaces, state)
		.mount_auth_endpoint(get_user_info, state)
}
