use axum::Router;

use crate::prelude::*;

/// The endpoint to invite a user, by email, to a workspace
mod invite_user_to_workspace;
mod list_users_in_workspace;
/// The endpoint to list the pending invites for a workspace
mod list_workspace_invites;
mod remove_user_from_workspace;
/// The endpoint to resend a pending workspace invite
mod resend_workspace_invite;
/// The endpoint to revoke a pending workspace invite
mod revoke_workspace_invite;
mod update_user_roles_in_workspace;
/// The endpoint to update the roles a pending invite will grant
mod update_workspace_invite_roles;

use self::{
	invite_user_to_workspace::*,
	list_users_in_workspace::*,
	list_workspace_invites::*,
	remove_user_from_workspace::*,
	resend_workspace_invite::*,
	revoke_workspace_invite::*,
	update_user_roles_in_workspace::*,
	update_workspace_invite_roles::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(invite_user_to_workspace, state, allowed_client_types)
		.mount_auth_endpoint(list_users_in_workspace, state, allowed_client_types)
		.mount_auth_endpoint(list_workspace_invites, state, allowed_client_types)
		.mount_auth_endpoint(remove_user_from_workspace, state, allowed_client_types)
		.mount_auth_endpoint(resend_workspace_invite, state, allowed_client_types)
		.mount_auth_endpoint(revoke_workspace_invite, state, allowed_client_types)
		.mount_auth_endpoint(update_user_roles_in_workspace, state, allowed_client_types)
		.mount_auth_endpoint(update_workspace_invite_roles, state, allowed_client_types)
}
