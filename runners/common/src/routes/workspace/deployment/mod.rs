use axum::Router;

mod create_deployment;
mod list_deployment;

use self::{create_deployment::*, list_deployment::*};
use crate::{prelude::*, utils::RouterExt};

#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor,
{
	Router::new().mount_endpoint(list_deployment, state)
	// .mount_endpoint(create_deployment, state)
}
