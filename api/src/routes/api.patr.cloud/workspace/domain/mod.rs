use axum::Router;

use crate::prelude::*;

mod add_domain_to_workspace;
mod delete_domain_in_workspace;
mod get_domain_info_in_workspace;
mod is_domain_valid;
mod list_domains_in_workspace;
mod verify_domain_in_workspace;

pub use self::{
	add_domain_to_workspace::*,
	delete_domain_in_workspace::*,
	get_domain_info_in_workspace::*,
	is_domain_valid::*,
	list_domains_in_workspace::*,
	verify_domain_in_workspace::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(add_domain_to_workspace, state, allowed_client_types)
		.mount_auth_endpoint(list_domains_in_workspace, state, allowed_client_types)
		.mount_auth_endpoint(delete_domain_in_workspace, state, allowed_client_types)
		.mount_auth_endpoint(is_domain_valid, state, allowed_client_types)
		.mount_auth_endpoint(get_domain_info_in_workspace, state, allowed_client_types)
		.mount_auth_endpoint(verify_domain_in_workspace, state, allowed_client_types)
}
