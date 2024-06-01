use axum::Router;

use crate::prelude::*;

mod get_current_permissions;
mod list_all_permissions;
mod list_all_resource_types;

pub use self::{get_current_permissions::*, list_all_permissions::*, list_all_resource_types::*};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(get_current_permissions, state)
		.mount_auth_endpoint(list_all_permissions, state)
		.mount_auth_endpoint(list_all_resource_types, state)
		.with_state(state.clone())
}
