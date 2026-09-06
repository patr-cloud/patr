use axum::Router;

use crate::prelude::*;

mod get_current_permissions;
mod list_all_permissions;
mod list_all_resource_types;

use self::{get_current_permissions::*, list_all_permissions::*, list_all_resource_types::*};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(get_current_permissions, state, allowed_client_types)
		.mount_auth_endpoint(list_all_permissions, state, allowed_client_types)
		.mount_auth_endpoint(list_all_resource_types, state, allowed_client_types)
		.with_state(state.clone())
}
