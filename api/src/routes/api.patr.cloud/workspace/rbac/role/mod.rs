use axum::Router;

use crate::prelude::*;

mod create_new_role;
mod delete_role;
mod get_role_info;
mod list_all_roles;
mod list_users_for_role;
mod update_role;

use self::{
	create_new_role::*,
	delete_role::*,
	get_role_info::*,
	list_all_roles::*,
	list_users_for_role::*,
	update_role::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(create_new_role, state, allowed_client_types)
		.mount_auth_endpoint(delete_role, state, allowed_client_types)
		.mount_auth_endpoint(get_role_info, state, allowed_client_types)
		.mount_auth_endpoint(list_all_roles, state, allowed_client_types)
		.mount_auth_endpoint(list_users_for_role, state, allowed_client_types)
		.mount_auth_endpoint(update_role, state, allowed_client_types)
		.with_state(state.clone())
}
