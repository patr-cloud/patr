use axum::Router;

mod create_deployment;
mod delete_deployment;
mod get_deployment_info;
mod list_deployment;
mod start_deployment;
mod stop_deployment;
mod update_deployment;

use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_info::*,
	list_deployment::*,
	update_deployment::*,
};
use crate::{prelude::*, utils::RouterExt};

#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Clone + 'static,
{
	Router::new()
		.mount_auth_endpoint(list_deployment, state)
		.mount_auth_endpoint(delete_deployment, state)
		.mount_auth_endpoint(update_deployment, state)
		.mount_auth_endpoint(create_deployment, state)
		.mount_auth_endpoint(get_deployment_info, state)
}
