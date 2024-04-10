use axum::{http::StatusCode, Router};
use models::{api::{workspace::infrastructure::database::*, WithId}, ErrorType};

use crate::prelude::*;

pub async fn get_database(
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

	let database = query!(
		r#"
		SELECT
			id,
			name,
			region,
			engine as "engine: DatabaseEngine",
			database_plan_id,
			status as "status: DatabaseStatus",
			username,
			num_nodes
		FROM
			managed_database
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		database_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|database| GetDatabaseResponse {
		database: WithId::new(
			database.id,
			Database {
				name: database.name,
				engine: database.engine.to_owned(),
				version: match database.engine {
					DatabaseEngine::Postgres => "12".to_string(),
					DatabaseEngine::Mysql => "8".to_string(),
					DatabaseEngine::Mongo => "4".to_string(),
					DatabaseEngine::Redis => "6".to_string(),
				},
				database_plan_id: database.database_plan_id.into(),
				region: database.region.into(),
				status: database.status,
				num_nodes: database.num_nodes as u16,
				public_connection: DatabaseConnectionInfo {
					host: format!("service-{0}", database.id),
					port: match database.engine {
						DatabaseEngine::Postgres => 5432,
						DatabaseEngine::Mysql => 3306,
						DatabaseEngine::Mongo => 27017,
						DatabaseEngine::Redis => 6379,
					},
					username: database.username,
					password: None,
				},
			}
		)
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(database)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}