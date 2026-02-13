use axum::Router;

use crate::prelude::*;

mod create_repository;
mod delete_repository;
mod delete_repository_image;
mod get_repository_image_details;
// mod get_repository_image_exposed_ports;
mod get_repository_info;
mod list_repositories;
mod list_repository_tags;

use self::{
	create_repository::*,
	delete_repository::*,
	delete_repository_image::*,
	get_repository_image_details::*,
	// get_repository_image_exposed_ports::*,
	get_repository_info::*,
	list_repositories::*,
	list_repository_tags::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(create_repository, state, allowed_client_type)
		.mount_auth_endpoint(delete_repository, state, allowed_client_type)
		.mount_auth_endpoint(delete_repository_image, state, allowed_client_type)
		.mount_auth_endpoint(get_repository_image_details, state, allowed_client_type)
		// .mount_auth_endpoint(
		// 	get_repository_image_exposed_ports,
		// 	state,
		// 	allowed_client_type,
		// )
		.mount_auth_endpoint(get_repository_info, state, allowed_client_type)
		.mount_auth_endpoint(list_repositories, state, allowed_client_type)
		.mount_auth_endpoint(list_repository_tags, state, allowed_client_type)
		.with_state(state.clone())
}
