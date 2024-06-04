use axum::Router;

use crate::prelude::*;

mod api_token;
mod change_password;
mod get_user_details;
mod get_user_info;
mod list_workspaces;
mod mfa;
#[allow(unreachable_code, unused_variables)]
mod recovery_options;
mod update_user_info;
#[allow(unreachable_code, unused_variables)]
mod web_logins;

pub use self::{
	change_password::*,
	get_user_details::*,
	get_user_info::*,
	list_workspaces::*,
	update_user_info::*,
};

/// Sets up the user routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.merge(api_token::setup_routes(state).await)
		.merge(mfa::setup_routes(state).await)
		.merge(recovery_options::setup_routes(state).await)
		.merge(web_logins::setup_routes(state).await)
		.mount_auth_endpoint(change_password, state)
		.mount_auth_endpoint(get_user_details, state)
		.mount_auth_endpoint(get_user_info, state)
		.mount_auth_endpoint(list_workspaces, state)
		.mount_auth_endpoint(update_user_info, state)
}
