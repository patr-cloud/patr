use axum::Router;

use crate::prelude::*;

mod add_runner_to_workspace;
mod get_ingress_token_for_runner;
mod get_runner_info;
mod get_runner_logs;
mod get_runner_metrics;
mod list_runners_for_workspace;
mod remove_runner_from_workspace;
mod stream_runner_data_for_workspace;
mod stream_runner_logs;
mod stream_runner_shell_connection;

use self::{
	add_runner_to_workspace::*,
	get_ingress_token_for_runner::*,
	get_runner_info::*,
	get_runner_logs::*,
	get_runner_metrics::*,
	list_runners_for_workspace::*,
	remove_runner_from_workspace::*,
	stream_runner_data_for_workspace::*,
	stream_runner_logs::*,
	stream_runner_shell_connection::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(add_runner_to_workspace, state, allowed_client_type)
		.mount_auth_endpoint(get_ingress_token_for_runner, state, allowed_client_type)
		.mount_auth_endpoint(get_runner_info, state, allowed_client_type)
		.mount_auth_endpoint(list_runners_for_workspace, state, allowed_client_type)
		.mount_auth_endpoint(remove_runner_from_workspace, state, allowed_client_type)
		.mount_auth_endpoint(stream_runner_data_for_workspace, state, allowed_client_type)
		.mount_auth_endpoint(get_runner_logs, state, allowed_client_type)
		.mount_auth_endpoint(get_runner_metrics, state, allowed_client_type)
		.mount_auth_endpoint(stream_runner_logs, state, allowed_client_type)
		.mount_auth_endpoint(stream_runner_shell_connection, state, allowed_client_type)
}
