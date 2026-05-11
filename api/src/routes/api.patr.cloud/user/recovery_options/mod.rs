mod update_user_email;
mod verify_user_email;

use axum::Router;

use self::{update_user_email::*, verify_user_email::*};
use crate::prelude::*;

/// Sets up the recovery options routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(update_user_email, state, allowed_client_type)
		.mount_auth_endpoint(verify_user_email, state, allowed_client_type)
}
