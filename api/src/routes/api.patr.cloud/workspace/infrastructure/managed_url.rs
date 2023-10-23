use crate::{prelude::*, service};
use axum::{http::StatusCode, Router};

use models::{
	api::workspace::infrastructure::managed_url::*,
	ApiRequest,
	ErrorType, prelude::WithId,
};

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(create_managed_url, state)
		.mount_endpoint(delete_managed_url, state)
		.mount_endpoint(list_managed_url, state)
		.mount_endpoint(update_managed_url, state)
		.mount_endpoint(verify_configuration, state)
		.with_state(state.clone())
}

async fn create_managed_url(
	AppRequest {
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
	}: AppRequest<'_, CreateManagedUrlRequest>,
) -> Result<AppResponse<CreateManagedUrlResponse>, ErrorType> {
	
	info!("Starting: Create managed URL");

	// LOGIC

    AppResponse::builder()
        .body(CreateManagedUrlResponse {
            id: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn delete_managed_url(
	AppRequest {
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
	}: AppRequest<'_, DeleteManagedUrlRequest>,
) -> Result<AppResponse<DeleteManagedUrlResponse>, ErrorType> {
	
	info!("Starting: Delete managed URL");

	// LOGIC

    AppResponse::builder()
        .body(DeleteManagedUrlResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn list_managed_url(
	AppRequest {
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
	}: AppRequest<'_, ListManagedUrlRequest>,
) -> Result<AppResponse<ListManagedUrlResponse>, ErrorType> {
	
	info!("Starting: List managed URL");

	// LOGIC

    AppResponse::builder()
        .body(ListManagedUrlResponse {
            urls: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn update_managed_url(
	AppRequest {
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
	}: AppRequest<'_, UpdateManagedUrlRequest>,
) -> Result<AppResponse<UpdateManagedUrlResponse>, ErrorType> {
	
	info!("Starting: Update managed URL");

	// LOGIC

    AppResponse::builder()
        .body(UpdateManagedUrlResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn verify_configuration(
	AppRequest {
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
	}: AppRequest<'_, VerifyManagedUrlConfigurationRequest>,
) -> Result<AppResponse<VerifyManagedUrlConfigurationResponse>, ErrorType> {
	
	info!("Starting: Verify configuration of managed URL");

	// LOGIC

    AppResponse::builder()
        .body(VerifyManagedUrlConfigurationResponse {
            configured: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}