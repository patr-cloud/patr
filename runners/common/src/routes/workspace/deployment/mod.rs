use axum::Router;

mod create_deployment;
mod delete_deployment;
mod list_deployment;
mod update_deployment;

use self::{create_deployment::*, delete_deployment::*, list_deployment::*, update_deployment::*};
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
}
