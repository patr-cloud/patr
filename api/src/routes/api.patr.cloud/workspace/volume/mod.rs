use axum::Router;

mod create_volume;
mod delete_volume;
mod get_volume_info;
mod list_volumes;
mod update_volume;

use self::{
	create_volume::*,
	delete_volume::*,
	get_volume_info::*,
	list_volumes::*,
	update_volume::*,
};
use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(create_volume, state, allowed_client_types)
		.mount_auth_endpoint(delete_volume, state, allowed_client_types)
		.mount_auth_endpoint(get_volume_info, state, allowed_client_types)
		.mount_auth_endpoint(list_volumes, state, allowed_client_types)
		.mount_auth_endpoint(update_volume, state, allowed_client_types)
}
