use axum::Router;

use crate::prelude::*;

mod add_runner_to_workspace;
mod get_runner_info;
mod list_runners_for_workspace;
mod remove_runner_from_workspace;
mod stream_runner_data_for_workspace;

use self::{
	add_runner_to_workspace::*,
	get_runner_info::*,
	list_runners_for_workspace::*,
	remove_runner_from_workspace::*,
	stream_runner_data_for_workspace::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(stream_runner_data_for_workspace, state)
		.mount_auth_endpoint(add_runner_to_workspace, state)
		.mount_auth_endpoint(remove_runner_from_workspace, state)
		.mount_auth_endpoint(list_runners_for_workspace, state)
		.mount_auth_endpoint(get_runner_info, state)
}
