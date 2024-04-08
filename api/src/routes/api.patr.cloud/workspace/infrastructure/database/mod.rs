mod create_database;
mod delete_database;
mod get_database;
mod list_database;


use axum::{http::StatusCode, Router};
use models::{api::{workspace::infrastructure::database::*, WithId}, ErrorType};
use crate::prelude::*;

use self::{
    create_database::*,
    delete_database::*,
    get_database::*,
    list_database::*,
};

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

	let plans = query!(
		r#"
		SELECT
			id,
			cpu,
			ram,
			volume
		FROM
			managed_database_plan;
		"#,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|plan| {
		WithId::new(
			plan.id.into(),
			DatabasePlan {
				cpu_count: plan.cpu,
				memory_count: plan.ram,
				volume: plan.volume
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListAllDatabaseMachineTypeResponse { plans })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}