use axum::{http::StatusCode, Router};
use models::{api::workspace::infrastructure::database::*, ErrorType};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(all_database_plan, state)
		.mount_auth_endpoint(create_database, state)
		.mount_auth_endpoint(delete_database, state)
		.mount_auth_endpoint(get_database, state)
		.mount_auth_endpoint(list_database, state)
		.with_state(state.clone())
}

async fn all_database_plan(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ListAllDatabaseMachineTypePath,
				query: (),
				headers: _,
				body: ListAllDatabaseMachineTypeRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ListAllDatabaseMachineTypeRequest>,
) -> Result<AppResponse<ListAllDatabaseMachineTypeRequest>, ErrorType> {
	info!("Starting: Get database plans");

	// LOGIC

	AppResponse::builder()
		.body(ListAllDatabaseMachineTypeResponse { plans: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn create_database(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateDatabasePath { workspace_id },
				query: (),
				headers: _,
				body:
					CreateDatabaseRequestProcessed {
						name,
						engine,
						database_plan_id,
						region,
						version,
						num_node,
					},
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, CreateDatabaseRequest>,
) -> Result<AppResponse<CreateDatabaseRequest>, ErrorType> {
	info!("Starting: Create database");

	// LOGIC

	AppResponse::builder()
		.body(CreateDatabaseResponse { id: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn delete_database(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteDatabasePath {
					workspace_id,
					database_id,
				},
				query: (),
				headers: _,
				body: DeleteDatabaseRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteDatabaseRequest>,
) -> Result<AppResponse<DeleteDatabaseRequest>, ErrorType> {
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
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDatabasePath {
					workspace_id,
					database_id,
				},
				query: (),
				headers: _,
				body: GetDatabaseRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDatabaseRequest>,
) -> Result<AppResponse<GetDatabaseRequest>, ErrorType> {
	info!("Starting: Get database");

	// LOGIC

	AppResponse::builder()
		.body(GetDatabaseResponse { database: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn list_database(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDatabasePath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers: _,
				body: ListDatabaseRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListDatabaseRequest>,
) -> Result<AppResponse<ListDatabaseRequest>, ErrorType> {
	info!("Starting: List database");

	// LOGIC

	AppResponse::builder()
		.body(ListDatabaseResponse { database: todo!() })
		.headers(ListDatabaseResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
