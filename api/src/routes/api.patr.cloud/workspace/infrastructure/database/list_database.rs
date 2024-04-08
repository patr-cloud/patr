use axum::{http::StatusCode, Router};
use models::{api::{workspace::infrastructure::database::*, WithId}, ErrorType};

use crate::prelude::*;

pub async fn list_database(
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

	let databases = query!(
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
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|database| {
		WithId::new(
			database.id.into(),
			Database {
				name: database.name,
				engine: database.engine.to_owned(),
				version: match database.engine {
					DatabaseEngine::Postgres => "12".to_string(),
					DatabaseEngine::Mysql => "8".to_string(),
					DatabaseEngine::Mongo => "4".to_string(),
					DatabaseEngine::Redis => "6".to_string(),
				},
				num_nodes: database.num_nodes as u16,
				database_plan_id: database.database_plan_id.into(),
				region: database.region.into(),
				status: database.status,
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
	.collect();

	AppResponse::builder()
		.body(ListDatabaseResponse { databases })
		.headers(ListDatabaseResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}