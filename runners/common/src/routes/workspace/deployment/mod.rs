use axum::Router;

/// The handler for creating a new deployment.
mod create_deployment;
/// The handler for deleting a deployment.
mod delete_deployment;
/// The handler for getting information about a deployment.
mod get_deployment_info;
/// The handler for getting the machine type supported by the deployments.
mod list_all_deployment_machine_types;
/// The handler for listing all deployments.
mod list_deployment;
/// The handler for starting a deployment.
mod start_deployment;
/// The handler for stopping a deployment.
mod stop_deployment;
/// The handler for updating a deployment.
mod update_deployment;

use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_info::*,
	list_all_deployment_machine_types::*,
	list_deployment::*,
	start_deployment::*,
	stop_deployment::*,
	update_deployment::*,
};
use crate::prelude::*;

/// Sets up the deployment routes
#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Send + 'static,
{
	Router::new()
		.mount_auth_json_endpoint(list_deployment, state)
		.mount_auth_json_endpoint(delete_deployment, state)
		.mount_auth_json_endpoint(update_deployment, state)
		.mount_auth_json_endpoint(create_deployment, state)
		.mount_auth_json_endpoint(get_deployment_info, state)
		.mount_auth_json_endpoint(start_deployment, state)
		.mount_auth_json_endpoint(stop_deployment, state)
		.mount_json_endpoint(list_all_deployment_machine_types, state)
}
