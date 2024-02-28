use axum::Router;

use crate::prelude::*;

mod activate_mfa;
mod change_password;
mod deactivate_mfa;
mod get_mfa_secret;
mod get_user_details;
mod get_user_info;
mod list_workspaces;
mod update_user_info;

pub use self::{
	activate_mfa::*,
	change_password::*,
	deactivate_mfa::*,
	get_mfa_secret::*,
	get_user_details::*,
	get_user_info::*,
	list_workspaces::*,
	update_user_info::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(change_password, state)
		.mount_auth_endpoint(get_user_details, state)
		.mount_auth_endpoint(get_user_info, state)
		.mount_auth_endpoint(list_workspaces, state)
		.mount_auth_endpoint(update_user_info, state)
		.mount_auth_endpoint(activate_mfa, state)
		.mount_auth_endpoint(deactivate_mfa, state)
		.mount_auth_endpoint(get_mfa_secret, state)
		.with_state(state.clone())
}
