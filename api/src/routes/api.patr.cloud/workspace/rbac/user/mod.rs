use axum::Router;

use crate::prelude::*;

mod list_users_in_workspace;
mod remove_user_from_workspace;
mod update_user_roles_in_workspace;

use self::{
	list_users_in_workspace::*,
	remove_user_from_workspace::*,
	update_user_roles_in_workspace::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(list_users_in_workspace, state)
		.mount_auth_endpoint(remove_user_from_workspace, state)
		.mount_auth_endpoint(update_user_roles_in_workspace, state)
}
