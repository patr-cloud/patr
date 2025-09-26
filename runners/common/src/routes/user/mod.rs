/// Get the user info that was registered with the user
mod get_user_info;
/// Get the list of workspaces that the user is a member of. For self hosted
/// mode, this is just the default workspace.
mod list_workspaces;

use axum::Router;

use self::{get_user_info::*, list_workspaces::*};
use crate::prelude::*;

/// Sets up the user routes
#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Send + 'static,
{
	Router::new()
		.mount_auth_endpoint(list_workspaces, state)
		.mount_auth_endpoint(get_user_info, state)
}
