use axum::{http::StatusCode, Router};
use models::{api::workspace::infrastructure::managed_url::*, ApiRequest, ErrorType};

use crate::prelude::*;

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

async fn create_managed_url(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, CreateManagedURLRequest>,
) -> Result<AppResponse<CreateManagedURLRequest>, ErrorType> {
	info!("Starting: Create managed URL");

	// LOGIC

	AppResponse::builder()
		.body(CreateManagedURLResponse { id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_managed_url(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, DeleteManagedURLRequest>,
) -> Result<AppResponse<DeleteManagedURLRequest>, ErrorType> {
	info!("Starting: Delete managed URL");

	// LOGIC

	AppResponse::builder()
		.body(DeleteManagedURLResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_managed_url(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, ListManagedURLRequest>,
) -> Result<AppResponse<ListManagedURLRequest>, ErrorType> {
	info!("Starting: List managed URL");

	// LOGIC

	AppResponse::builder()
		.body(ListManagedURLResponse { urls: todo!() })
		.headers(ListManagedURLResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn update_managed_url(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
	}: AuthenticatedAppRequest<'_, UpdateManagedURLRequest>,
) -> Result<AppResponse<UpdateManagedURLRequest>, ErrorType> {
	info!("Starting: Update managed URL");

	// LOGIC

	AppResponse::builder()
		.body(UpdateManagedURLResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn verify_configuration(
	AuthenticatedAppRequest {
		request: ApiRequest {
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
