mod delete_web_login;
mod get_web_login_info;
mod list_web_logins;

use axum::Router;

use self::{delete_web_login::*, get_web_login_info::*, list_web_logins::*};
use crate::prelude::*;

/// Sets up the web logins routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(delete_web_login, state, host_client_types)
		.mount_auth_endpoint(get_web_login_info, state, host_client_types)
		.mount_auth_endpoint(list_web_logins, state, host_client_types)
}
