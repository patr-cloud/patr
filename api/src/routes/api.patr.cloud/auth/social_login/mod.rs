use axum::Router;

use crate::prelude::*;

/// GitHub OAuth2 SSO routes
mod github;

/// Sets up the social-login routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new().merge(github::setup_routes(state, allowed_client_type).await)
}
