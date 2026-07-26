use axum::Router;

use crate::prelude::*;

mod approve_runner_link;
mod create_runner_link;
mod get_ingress_token_for_runner;
mod get_runner_info;
mod get_runner_link;
mod get_runner_logs;
mod get_runner_metrics;
mod list_runners_for_workspace;
mod reconnect_runner_link;
mod remove_runner_from_workspace;
mod stream_runner_data_for_workspace;
mod stream_runner_logs;
mod verify_runner_link;

use self::{
	approve_runner_link::*,
	create_runner_link::*,
	get_ingress_token_for_runner::*,
	get_runner_info::*,
	get_runner_link::*,
	get_runner_logs::*,
	get_runner_metrics::*,
	list_runners_for_workspace::*,
	reconnect_runner_link::*,
	remove_runner_from_workspace::*,
	stream_runner_data_for_workspace::*,
	stream_runner_logs::*,
	verify_runner_link::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(get_ingress_token_for_runner, state, allowed_client_types)
		.mount_auth_endpoint(get_runner_info, state, allowed_client_types)
		.mount_auth_endpoint(list_runners_for_workspace, state, allowed_client_types)
		.mount_auth_endpoint(remove_runner_from_workspace, state, allowed_client_types)
		.mount_auth_endpoint(
			stream_runner_data_for_workspace,
			state,
			allowed_client_types,
		)
		.mount_auth_endpoint(get_runner_logs, state, allowed_client_types)
		.mount_auth_endpoint(get_runner_metrics, state, allowed_client_types)
		.mount_auth_endpoint(stream_runner_logs, state, allowed_client_types)
		.mount_auth_endpoint(create_runner_link, state, allowed_client_types)
		.mount_auth_endpoint(verify_runner_link, state, allowed_client_types)
		.mount_auth_endpoint(get_runner_link, state, allowed_client_types)
		.mount_auth_endpoint(approve_runner_link, state, allowed_client_types)
		.mount_auth_endpoint(reconnect_runner_link, state, allowed_client_types)
}
