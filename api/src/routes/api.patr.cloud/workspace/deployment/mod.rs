use axum::Router;

mod create_deployment;
mod delete_deployment;
mod get_deployment_info;
mod get_deployment_log;
mod get_deployment_metric;
mod list_all_deployment_machine_types;
mod list_deployment;
mod list_deployment_history;
mod start_deployment;
mod stop_deployment;
mod update_deployment;

use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_info::*,
	get_deployment_log::*,
	get_deployment_metric::*,
	list_all_deployment_machine_types::*,
	list_deployment::*,
	list_deployment_history::*,
	start_deployment::*,
	stop_deployment::*,
	update_deployment::*,
};
use crate::prelude::*;

/*
Figure out how to structure:
	- ports
	- env vars
	- config mounts
	- Volume
	- deploy history
	- logs
	- metrics
*/

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(machine_type, state)
		.mount_auth_endpoint(list_deployment, state)
		.mount_auth_endpoint(list_deployment_history, state)
		.mount_auth_endpoint(create_deployment, state)
		.mount_auth_endpoint(get_deployment_info, state)
		.mount_auth_endpoint(start_deployment, state)
		.mount_auth_endpoint(stop_deployment, state)
		.mount_auth_endpoint(get_deployment_log, state)
		.mount_auth_endpoint(delete_deployment, state)
		.mount_auth_endpoint(update_deployment, state)
		.mount_auth_endpoint(get_deployment_metric, state)
}
