use axum::Router;

use crate::prelude::*;

/// The endpoint to accept a workspace invite
mod accept_workspace_invite;
mod api_token;
mod change_password;
mod get_user_details;
mod get_user_info;
mod list_workspaces;
mod mfa;
/// The endpoint to preview a workspace invite before accepting
mod preview_workspace_invite;
#[cfg(feature = "cloud")]
mod social_logins;
mod update_user_info;
#[expect(unused_variables)]
mod web_logins;

use self::{
	accept_workspace_invite::*,
	change_password::*,
	get_user_details::*,
	get_user_info::*,
	list_workspaces::*,
	preview_workspace_invite::*,
	update_user_info::*,
};

/// Sets up the user routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.merge(api_token::setup_routes(state, allowed_client_type).await)
		.merge(mfa::setup_routes(state, allowed_client_type).await)
		.merge(
			#[cfg(feature = "cloud")]
			{
				social_logins::setup_routes(state, allowed_client_type).await
			},
			#[cfg(not(feature = "cloud"))]
			{
				Router::new()
			},
		)
		.merge(web_logins::setup_routes(state, allowed_client_type).await)
		.mount_auth_endpoint(accept_workspace_invite, state, allowed_client_type)
		.mount_auth_endpoint(preview_workspace_invite, state, allowed_client_type)
		.mount_auth_endpoint(change_password, state, allowed_client_type)
		.mount_auth_endpoint(get_user_details, state, allowed_client_type)
		.mount_auth_endpoint(get_user_info, state, allowed_client_type)
		.mount_auth_endpoint(list_workspaces, state, allowed_client_type)
		.mount_auth_endpoint(update_user_info, state, allowed_client_type)
}
