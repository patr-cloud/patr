use axum::Router;

use crate::prelude::*;

mod api_token;
mod change_password;
mod get_user_details;
mod get_user_info;
mod list_workspaces;
mod mfa;
#[expect(unused_variables)]
mod recovery_options;
mod update_user_info;
#[expect(unused_variables)]
mod web_logins;

use self::{
	change_password::*,
	get_user_details::*,
	get_user_info::*,
	list_workspaces::*,
	update_user_info::*,
};

/// Sets up the user routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.merge(api_token::setup_routes(state, allowed_client_type).await)
		.merge(mfa::setup_routes(state, allowed_client_type).await)
		.merge(recovery_options::setup_routes(state, allowed_client_type).await)
		.merge(web_logins::setup_routes(state, allowed_client_type).await)
		.mount_auth_endpoint(change_password, state, allowed_client_type)
		.mount_auth_endpoint(get_user_details, state, allowed_client_type)
		.mount_auth_endpoint(get_user_info, state, allowed_client_type)
		.mount_auth_endpoint(list_workspaces, state, allowed_client_type)
		.mount_auth_endpoint(update_user_info, state, allowed_client_type)
}
