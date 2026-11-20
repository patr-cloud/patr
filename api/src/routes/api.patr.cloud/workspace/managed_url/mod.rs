use axum::Router;

use crate::prelude::*;

mod create_managed_url;
mod delete_managed_url;
mod get_managed_url_info;
mod list_managed_url;
mod update_managed_url;
mod verify_configuration;

use self::{
	create_managed_url::*,
	delete_managed_url::*,
	get_managed_url_info::*,
	list_managed_url::*,
	update_managed_url::*,
	verify_configuration::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(create_managed_url, state, host_client_types)
		.mount_auth_endpoint(delete_managed_url, state, host_client_types)
		.mount_auth_endpoint(get_managed_url_info, state, host_client_types)
		.mount_auth_endpoint(list_managed_url, state, host_client_types)
		.mount_auth_endpoint(update_managed_url, state, host_client_types)
		.mount_auth_endpoint(verify_configuration, state, host_client_types)
}
