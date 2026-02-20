use axum::Router;

use crate::prelude::*;

mod create_repository;
mod delete_repository;
mod delete_repository_manifest;
mod get_exposed_ports;
mod get_repository_info;
mod get_repository_manifest_details;
mod list_repositories;
mod list_repository_manifests;
mod list_repository_tags;

use self::{
	create_repository::*,
	delete_repository::*,
	delete_repository_manifest::*,
	get_exposed_ports::*,
	get_repository_info::*,
	get_repository_manifest_details::*,
	list_repositories::*,
	list_repository_manifests::*,
	list_repository_tags::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(create_repository, state, allowed_client_type)
		.mount_auth_endpoint(delete_repository_manifest, state, allowed_client_type)
		.mount_auth_endpoint(delete_repository, state, allowed_client_type)
		.mount_auth_endpoint(get_exposed_ports, state, allowed_client_type)
		.mount_auth_endpoint(get_repository_manifest_details, state, allowed_client_type)
		.mount_auth_endpoint(get_repository_info, state, allowed_client_type)
		.mount_auth_endpoint(list_repositories, state, allowed_client_type)
		.mount_auth_endpoint(list_repository_manifests, state, allowed_client_type)
		.mount_auth_endpoint(list_repository_tags, state, allowed_client_type)
		.with_state(state.clone())
}
