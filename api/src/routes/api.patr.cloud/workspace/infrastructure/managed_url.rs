use crate::prelude::*;
use axum::{http::StatusCode, Router};

use models::{
	api::workspace::infrastructure::managed_url::*,
	ApiRequest,
	ErrorType,
};

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
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
	}: AuthenticatedAppRequest<'_, CreateManagedUrlRequest>,
) -> Result<AppResponse<CreateManagedUrlRequest>, ErrorType> {
	
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
	}: AuthenticatedAppRequest<'_, DeleteManagedUrlRequest>,
) -> Result<AppResponse<DeleteManagedUrlRequest>, ErrorType> {
	
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
	}: AuthenticatedAppRequest<'_, ListManagedUrlRequest>,
) -> Result<AppResponse<ListManagedUrlRequest>, ErrorType> {
	
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
	}: AuthenticatedAppRequest<'_, UpdateManagedUrlRequest>,
) -> Result<AppResponse<UpdateManagedUrlRequest>, ErrorType> {
	
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
	}: AuthenticatedAppRequest<'_, VerifyManagedUrlConfigurationRequest>,
) -> Result<AppResponse<VerifyManagedUrlConfigurationRequest>, ErrorType> {
	
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