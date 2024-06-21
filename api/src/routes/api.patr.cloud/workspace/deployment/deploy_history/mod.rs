use axum::Router;

use crate::prelude::*;

mod delete_deploy_history;
mod list_deploy_history;

use self::{delete_deploy_history::*, list_deploy_history::*};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(list_deploy_history, state)
		.mount_auth_endpoint(delete_deploy_history, state)
}
