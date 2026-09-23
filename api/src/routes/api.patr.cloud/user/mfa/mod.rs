mod activate_mfa;
mod deactivate_mfa;
mod get_mfa_secret;

use axum::Router;

use self::{activate_mfa::*, deactivate_mfa::*, get_mfa_secret::*};
use crate::prelude::*;

/// Sets up the MFA routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(activate_mfa, state, host_client_types)
		.mount_auth_endpoint(deactivate_mfa, state, host_client_types)
		.mount_auth_endpoint(get_mfa_secret, state, host_client_types)
}
