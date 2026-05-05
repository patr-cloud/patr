use axum::Router;

use crate::prelude::*;

/// Handler for `POST /auth/social-login/github/callback` — completes the OAuth
/// flow
mod callback;
/// Handler for `POST /auth/social-login/github` — initiates the OAuth flow
mod initiate;
/// Handler for `POST /auth/social-login/github/setup` — creates a new Patr
/// account from a GitHub identity
mod setup;

use self::{callback::*, initiate::*, setup::*};

/// Sets up the GitHub social-login routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_endpoint(github_oauth_initiate, state, allowed_client_type)
		.mount_endpoint(github_oauth_callback, state, allowed_client_type)
		.mount_endpoint(github_oauth_setup, state, allowed_client_type)
}
