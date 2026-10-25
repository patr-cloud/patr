use axum::Router;

use crate::prelude::*;

mod create_repository;
mod delete_repository;
mod delete_repository_manifest;
mod get_exposed_ports;
mod get_registry_usage;
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
	get_registry_usage::*,
	get_repository_info::*,
	get_repository_manifest_details::*,
	list_repositories::*,
	list_repository_manifests::*,
	list_repository_tags::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(create_repository, state, host_client_types)
		.mount_auth_endpoint(delete_repository_manifest, state, host_client_types)
		.mount_auth_endpoint(delete_repository, state, host_client_types)
		.mount_auth_endpoint(get_exposed_ports, state, host_client_types)
		.mount_auth_endpoint(get_registry_usage, state, host_client_types)
		.mount_auth_endpoint(get_repository_manifest_details, state, host_client_types)
		.mount_auth_endpoint(get_repository_info, state, host_client_types)
		.mount_auth_endpoint(list_repositories, state, host_client_types)
		.mount_auth_endpoint(list_repository_manifests, state, host_client_types)
		.mount_auth_endpoint(list_repository_tags, state, host_client_types)
		.with_state(state.clone())
}
