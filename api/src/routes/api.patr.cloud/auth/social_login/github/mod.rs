use axum::Router;

use crate::prelude::*;

/// Handler for `POST /auth/social-login/{provider}/callback` — completes the
/// social-login OAuth flow
mod callback;
/// Handler for `POST /auth/social-login/{provider}` — initiates the
/// social-login OAuth flow
mod initiate;
/// Handler for `POST /auth/social-login/{provider}/setup` — creates a new
/// Patr account from a social-login identity
mod setup;

use self::{callback::*, initiate::*, setup::*};

/// Sets up the social-login routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_endpoint(social_login_initiate, state, allowed_client_type)
		.mount_endpoint(social_login_callback, state, allowed_client_type)
		.mount_endpoint(social_login_setup, state, allowed_client_type)
}
