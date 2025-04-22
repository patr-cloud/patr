use axum::Router;

use crate::prelude::*;

/// The handler to login a user
mod login;
/// The handler to sign up a user
mod sign_up;

use self::{login::*, sign_up::*};

/// Sets up the auth routes
#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Send + 'static,
{
	Router::new()
		.mount_json_endpoint(login, state)
		.mount_json_endpoint(sign_up, state)
}
