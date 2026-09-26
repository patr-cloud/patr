use axum::Router;

mod create_secret;
mod delete_secret;
mod get_secret_info;
mod list_secrets_for_workspace;
mod update_secret;

use self::{
	create_secret::*,
	delete_secret::*,
	get_secret_info::*,
	list_secrets_for_workspace::*,
	update_secret::*,
};
use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(create_secret, state, allowed_client_type)
		.mount_auth_endpoint(delete_secret, state, allowed_client_type)
		.mount_auth_endpoint(get_secret_info, state, allowed_client_type)
		.mount_auth_endpoint(list_secrets_for_workspace, state, allowed_client_type)
		.mount_auth_endpoint(update_secret, state, allowed_client_type)
		.with_state(state.clone())
}
