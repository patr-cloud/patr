use axum::Router;

use crate::prelude::*;

mod delete_deploy_history;
mod list_deploy_history;
mod revert_deployment;

use self::{delete_deploy_history::*, list_deploy_history::*, revert_deployment::*};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(list_deploy_history, state, host_client_types)
		.mount_auth_endpoint(delete_deploy_history, state, host_client_types)
		.mount_auth_endpoint(revert_deployment, state, host_client_types)
}
