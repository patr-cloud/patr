/// The endpoint to list the apps acting on a user's behalf
mod list_oauth_grants;
/// The endpoint to revoke an app's access
mod revoke_oauth_grant;

use axum::Router;

use self::{list_oauth_grants::*, revoke_oauth_grant::*};
use crate::prelude::*;

/// Sets up the OAuth grant routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(list_oauth_grants, state, host_client_types)
		.mount_auth_endpoint(revoke_oauth_grant, state, host_client_types)
}
