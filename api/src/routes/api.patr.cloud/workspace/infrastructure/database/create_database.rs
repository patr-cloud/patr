use axum::{http::StatusCode, Router};
use models::{api::{workspace::{infrastructure::database::*, region::RegionStatus}, WithId}, ErrorType};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn create_database(
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

	let database_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut **database)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	// Check if region active or not
	let region_details = query!(
		r#"
		SELECT
			status as "status: RegionStatus",
			workspace_id
		FROM
			region
		WHERE
			id = $1;
		"#,
		region as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.filter(|region| todo!("return if patr region or if workspace_id is some"))
	.ok_or(ErrorType::server_error("Could not get region details"))?;

	if !(region_details.status == RegionStatus::Active || todo!("Check if patr region")) {
		return Err(ErrorType::RegionNotActive);
	}

	todo!("Check creation limit");

	query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			($1, (SELECT id FROM resource_type WHERE name = 'database'), $2, $3);
		"#,
		database_id as _,
		workspace_id as _,
		OffsetDateTime::now_utc()
	)
	.execute(&mut **database)
	.await?;

	todo!("If not byoc region, start database usage history");

	let password = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	let username = match engine {
		DatabaseEngine::Postgres => "postgres",
		DatabaseEngine::Mysql => "root",
		DatabaseEngine::Mongo => "root",
		DatabaseEngine::Redis => "root",
	};

	// Create entry in database
	query!(
		r#"
		INSERT INTO
			managed_database(
				id,
				name,
				workspace_id,
				region,
				status,
				engine,
				database_plan_id,
				username
			)
		VALUES
			($1, $2, $3, $4, 'creating', $5, $6, $7);
		"#,
		database_id as _,
		name as _,
		workspace_id as _,
		region as _,
		engine as _,
		database_plan_id as _,
		username,
	)
	.execute(&mut **database)
	.await?;

	todo!("Internal metrics");

	AppResponse::builder()
		.body(CreateDatabaseResponse { id: WithId::new(database_id, ())})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}