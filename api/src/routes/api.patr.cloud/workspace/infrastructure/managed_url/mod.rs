use axum::{http::StatusCode, Router};
use models::{api::workspace::infrastructure::managed_url::*, prelude::*};

use crate::prelude::*;

mod create_managed_url;
mod delete_managed_url;
mod list_managed_url;
mod update_managed_url;
mod verify_configuration;

use self::{
	create_managed_url::*,
	delete_managed_url::*,
	list_managed_url::*,
	update_managed_url::*,
	verify_configuration::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(create_managed_url, state)
		.mount_auth_endpoint(delete_managed_url, state)
		.mount_auth_endpoint(list_managed_url, state)
		.mount_auth_endpoint(update_managed_url, state)
		.mount_auth_endpoint(verify_configuration, state)
		.with_state(state.clone())
}

async fn verify_configuration(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, VerifyManagedURLConfigurationRequest>,
) -> Result<AppResponse<VerifyManagedURLConfigurationRequest>, ErrorType> {
	info!("Starting: Verify configuration of managed URL");

	// LOGIC

	AppResponse::builder()
		.body(VerifyManagedURLConfigurationResponse {
			configured: todo!(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
