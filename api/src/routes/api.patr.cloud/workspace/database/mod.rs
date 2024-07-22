use axum::{http::StatusCode, Router};
use models::{api::workspace::database::*, utils::TotalCountHeader, ErrorType};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(all_database_plan, state)
		.mount_auth_endpoint(create_database, state)
		.mount_auth_endpoint(delete_database, state)
		.mount_auth_endpoint(get_database, state)
		.mount_auth_endpoint(list_database, state)
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
	let id = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4a5a6a7d8")?;

	AppResponse::builder()
		.body(ListAllDatabaseMachineTypeResponse {
			plans: vec![WithId::new(
				id,
				DatabasePlan {
					cpu_count: 1,
					memory_count: 1024,
					volume: 1,
				},
			)],
		})
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
	let id = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;

	AppResponse::builder()
		.body(CreateDatabaseResponse {
			id: WithId::new(id, ()),
		})
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
	let id = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
	let region_id = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
	let plan_id = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4a5a6a7d8")?;

	AppResponse::builder()
		.body(GetDatabaseResponse {
			database: WithId::new(
				id,
				Database {
					name: "test-database".to_string(),
					engine: DatabaseEngine::Postgres,
					version: "14".to_string(),
					num_nodes: 2,
					database_plan_id: plan_id,
					region: region_id,
					status: DatabaseStatus::Creating,
					public_connection: models::api::workspace::database::DatabaseConnection {
						host: "132.12.12.1".to_string(),
						port: 5432,
						username: "root".to_string(),
						password: "password".to_string(),
					},
				},
			),
		})
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
	let id_1 = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
	let plan_id = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;
	let region_id = Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8")?;

	AppResponse::builder()
		.body(ListDatabaseResponse {
			database: vec![WithId::new(
				id_1,
				Database {
					name: "test-database".to_string(),
					engine: DatabaseEngine::Postgres,
					version: "14".to_string(),
					num_nodes: 2,
					database_plan_id: plan_id,
					region: region_id,
					status: DatabaseStatus::Creating,
					public_connection: models::api::workspace::database::DatabaseConnection {
						host: "132.12.12.1".to_string(),
						port: 5432,
						username: "root".to_string(),
						password: "password".to_string(),
					},
				},
			)],
		})
		.headers(ListDatabaseResponseHeaders {
			total_count: TotalCountHeader(2),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
