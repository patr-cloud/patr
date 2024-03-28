use axum::Router;

use crate::prelude::*;

#[allow(unused_variables, dead_code, unreachable_code, unused_imports)]
mod domain;
mod infrastructure;
#[allow(unused_variables, dead_code, unreachable_code, unused_imports)]
mod rbac;
#[allow(unused_variables, dead_code, unreachable_code, unused_imports)]
mod region;
#[allow(unused_variables, dead_code, unreachable_code, unused_imports)]
mod runner;
#[allow(unused_variables, dead_code, unreachable_code, unused_imports)]
mod secret;
mod container_registry;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.merge(infrastructure::setup_routes(state).await)
		.merge(domain::setup_routes(state).await)
		.merge(region::setup_routes(state).await)
		.merge(secret::setup_routes(state).await)
		.merge(rbac::setup_routes(state).await)
		.merge(runner::setup_routes(state).await)
		.merge(container_registry::setup_routes(state).await)
}
