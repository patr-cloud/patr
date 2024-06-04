use axum::Router;

use crate::prelude::*;

// mod container_registry;
#[allow(unreachable_code, unused_variables)]
mod database;
#[allow(unreachable_code, unused_variables)]
mod deployment;
#[allow(unreachable_code, unused_variables)]
mod domain;
mod managed_url;
mod rbac;
mod runner;
#[allow(unreachable_code, unused_variables)]
mod secret;
// mod static_site;

mod create_workspace;
mod delete_workspace;
mod get_workspace_info;
mod is_name_available;
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
		// .merge(static_site::setup_routes(state).await)
		.mount_auth_endpoint(create_workspace, state)
		.mount_auth_endpoint(delete_workspace, state)
		.mount_auth_endpoint(get_workspace_info, state)
		.mount_auth_endpoint(is_name_available, state)
		.mount_auth_endpoint(update_workspace_info, state)
}
