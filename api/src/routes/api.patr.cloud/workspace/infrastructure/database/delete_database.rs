use axum::{http::StatusCode, Router};
use models::{api::workspace::infrastructure::database::*, ErrorType};
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn delete_database(
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

	// Check if database exists
	let deployment = query!(
		r#"
		SELECT
			id
		FROM
			managed_database
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		database_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	query!(
		r#"
		UPDATE
			managed_database
		SET
			deleted = $2,
			status = 'deleted'
		WHERE
			id = $1;
		"#,
		database_id as _,
		OffsetDateTime::now_utc()
	)
	.execute(&mut **database)
	.await?;

	todo!("Stop usage history");
	todo!("Internal metrics");

	AppResponse::builder()
		.body(DeleteDatabaseResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}