/// The endpoint to list every app acting on the user's behalf
mod list_oauth_grants;
/// The endpoint to revoke a single grant
mod revoke_oauth_grant;
/// The endpoint to revoke every grant for one app
mod revoke_oauth_grants_for_client;

use axum::Router;

use self::{list_oauth_grants::*, revoke_oauth_grant::*, revoke_oauth_grants_for_client::*};
use crate::prelude::*;

/// Sets up the OAuth grant management routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(list_oauth_grants, state, allowed_client_type)
		.mount_auth_endpoint(revoke_oauth_grant, state, allowed_client_type)
		.mount_auth_endpoint(revoke_oauth_grants_for_client, state, allowed_client_type)
}
