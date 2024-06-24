use axum::Router;

use crate::prelude::*;

// mod container_registry;
#[allow(unreachable_code, unused_variables)]
mod database;
mod deployment;
#[allow(unreachable_code, unused_variables)]
mod domain;
mod managed_url;
mod rbac;
mod runner;
#[allow(unreachable_code, unused_variables)]
mod secret;
#[allow(unreachable_code, unused_variables)]
mod static_site;
mod volume;

/// The handler to create a new workspace. The workspace name must be unique.
mod create_workspace;
/// The handler to delete a workspace. This will delete all associated data
/// with the workspace, including the database, container registry, and any
/// other resources. This is a destructive operation and cannot be undone.
/// The workspace must be empty before it can be deleted.
mod delete_workspace;
/// The handler to get the information of a workspace. This includes the
/// workspace's name, the user who created it, and the date it was created.
mod get_workspace_info;
/// The handler to check if a workspace name is available. This is used when
/// creating a new workspace to ensure that the name is unique.
mod is_name_available;
/// The handler to update the information of a workspace. At the moment, only
/// the name can be updated. However, this will be expanded in the future. At
/// least one parameter must be provided for the update.
mod update_workspace_info;

use self::{
	create_workspace::*,
	delete_workspace::*,
	get_workspace_info::*,
	is_name_available::*,
	update_workspace_info::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		// .merge(container_registry::setup_routes(state).await)
		.merge(domain::setup_routes(state).await)
		.merge(database::setup_routes(state).await)
		.merge(deployment::setup_routes(state).await)
		.merge(managed_url::setup_routes(state).await)
		.merge(rbac::setup_routes(state).await)
		.merge(runner::setup_routes(state).await)
		.merge(secret::setup_routes(state).await)
		.merge(static_site::setup_routes(state).await)
		.merge(volume::setup_routes(state).await)
		.mount_auth_endpoint(create_workspace, state)
		.mount_auth_endpoint(delete_workspace, state)
		.mount_auth_endpoint(get_workspace_info, state)
		.mount_auth_endpoint(is_name_available, state)
		.mount_auth_endpoint(update_workspace_info, state)
}
