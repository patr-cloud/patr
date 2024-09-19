use axum::Router;

use crate::{prelude::*, utils::RouterExt};

mod login;
mod sign_up;

use self::{login::*, sign_up::*};

#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Clone + 'static,
{
	Router::new()
		.mount_endpoint(login, state)
		.mount_endpoint(sign_up, state)
}