use axum::Router;

use crate::prelude::*;

/// Handler for `POST /user/social-login/github/callback` — completes
/// the authenticated "Connect GitHub" flow
mod callback;
/// Handler for `POST /user/social-login/github/connect` — initiates the
/// authenticated "Connect GitHub" flow
mod connect;
/// Handler for `DELETE /user/social-login/{provider}` — removes the link
mod disconnect;
/// Handler for `GET /user/social-login` — lists linked providers
mod list;

use self::{callback::*, connect::*, disconnect::*, list::*};

/// Sets up the social-login management routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(list_social_logins, state, host_client_types)
		.mount_auth_endpoint(disconnect_social_login, state, host_client_types)
		.mount_auth_endpoint(connect_social_login_initiate, state, host_client_types)
		.mount_auth_endpoint(social_login_callback, state, host_client_types)
}
