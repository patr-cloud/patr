use axum::Router;

use crate::prelude::*;

mod create_service_account;
mod delete_service_account;
mod get_service_account_info;
mod list_service_accounts;
mod regenerate_service_account_token;
mod update_service_account;

use self::{
	create_service_account::*,
	delete_service_account::*,
	get_service_account_info::*,
	list_service_accounts::*,
	regenerate_service_account_token::*,
	update_service_account::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_types: &[ClientType]) -> Router {
	Router::new()
		.mount_auth_endpoint(create_service_account, state, allowed_client_types)
		.mount_auth_endpoint(list_service_accounts, state, allowed_client_types)
		.mount_auth_endpoint(get_service_account_info, state, allowed_client_types)
		.mount_auth_endpoint(update_service_account, state, allowed_client_types)
		.mount_auth_endpoint(delete_service_account, state, allowed_client_types)
		.mount_auth_endpoint(
			regenerate_service_account_token,
			state,
			allowed_client_types,
		)
}
