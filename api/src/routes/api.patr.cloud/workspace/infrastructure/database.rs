use crate::{prelude::*, service};
use axum::{http::StatusCode, Router};

use models::{
	api::workspace::infrastructure::database::*,
	ApiRequest,
	ErrorType, prelude::WithId,
};

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(create_database, state)
		.with_state(state.clone());

	Router::new()
		.mount_endpoint(delete_database, state)
		.with_state(state.clone());
		
	Router::new()
		.mount_endpoint(get_database, state)
		.with_state(state.clone());

	Router::new()
		.mount_endpoint(list_database, state)
		.with_state(state.clone())
}

async fn create_database(
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
	}: AppRequest<'_, CreateDatabaseRequest>,
) -> Result<AppResponse<CreateDatabaseResponse>, ErrorType> {
	
	info!("Starting: Create database");

	// LOGIC

    AppResponse::builder()
        .body(CreateDatabaseResponse {
            id: todo!(),
        })
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn delete_database(
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
	}: AppRequest<'a, DeleteDatabaseRequest>,
) -> Result<AppResponse<DeleteDatabaseResponse>, ErrorType> {

	info!("Starting: Delete database");

	// LOGIC

	AppResponse::builder()
        .body(DeleteDatabaseResponse)
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result() 
}

async fn get_database(
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
	}: AppRequest<'a, GetDatabaseRequest>,
) -> Result<AppResponse<GetDatabaseResponse>, ErrorType> {

	info!("Starting: Get database");

	// LOGIC

	AppResponse::builder()
        .body(GetDatabaseResponse{
			database: todo!()
		})
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result()
}

async fn list_database(
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
	}: AppRequest<'a, ListDatabaseRequest>,
) -> Result<AppResponse<ListDatabaseResponse>, ErrorType> {

	info!("Starting: List database");

	// LOGIC

	AppResponse::builder()
        .body(ListDatabaseResponse{
			database: todo!()
		})
        .headers(())
        .status_code(StatusCode::OK)
        .build()
        .into_result()
}