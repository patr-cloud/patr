use axum::Router;

/// The history of deploys for a deployment. This includes the status of the
/// deploy, and the time it was deployed.
pub mod deploy_history;

mod create_deployment;
mod delete_deployment;
mod get_deployment_info;
mod get_deployment_logs;
mod get_deployment_metric;
mod list_all_deployment_machine_types;
mod list_deployment;
mod start_deployment;
mod stop_deployment;
mod stream_deployment_logs;
mod update_deployment;

use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_info::*,
	get_deployment_logs::*,
	get_deployment_metric::*,
	list_all_deployment_machine_types::*,
	list_deployment::*,
	start_deployment::*,
	stop_deployment::*,
	stream_deployment_logs::*,
	update_deployment::*,
};
use crate::prelude::*;

/*
Figure out how to structure:
	- Volume
	- logs
	- metrics
	- backups
*/

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.merge(deploy_history::setup_routes(state, allowed_client_type).await)
		.mount_endpoint(machine_type, state, allowed_client_type)
		.mount_auth_endpoint(list_deployment, state, allowed_client_type)
		.mount_auth_endpoint(create_deployment, state, allowed_client_type)
		.mount_auth_endpoint(get_deployment_info, state, allowed_client_type)
		.mount_auth_endpoint(start_deployment, state, allowed_client_type)
		.mount_auth_endpoint(stop_deployment, state, allowed_client_type)
		.mount_auth_endpoint(get_deployment_logs, state, allowed_client_type)
		.mount_auth_endpoint(delete_deployment, state, allowed_client_type)
		.mount_auth_endpoint(update_deployment, state, allowed_client_type)
		.mount_auth_endpoint(get_deployment_metric, state, allowed_client_type)
		.mount_auth_endpoint(stream_deployment_logs, state, allowed_client_type)
}
