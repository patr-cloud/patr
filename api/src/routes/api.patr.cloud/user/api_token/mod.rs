mod create_api_token;
mod get_api_token_info;
mod list_api_tokens;
mod regenerate_api_token;
mod revoke_api_token;
mod update_api_token;

use axum::Router;

pub use self::{
	create_api_token::*,
	get_api_token_info::*,
	list_api_tokens::*,
	regenerate_api_token::*,
	revoke_api_token::*,
	update_api_token::*,
};
use crate::prelude::*;

/// Sets up the api-token routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(create_api_token, state)
		.mount_auth_endpoint(get_api_token_info, state)
		.mount_auth_endpoint(list_api_tokens, state)
		.mount_auth_endpoint(regenerate_api_token, state)
		.mount_auth_endpoint(revoke_api_token, state)
		.mount_auth_endpoint(update_api_token, state)
}
