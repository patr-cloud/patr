use axum::http::StatusCode;
use models::api::workspace::volume::*;

use crate::prelude::*;

pub async fn delete_volume(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteVolumePath {
					workspace_id: _,
					volume_id,
				},
				query: (),
				headers:
					DeleteVolumeRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteVolumeRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteVolumeRequest>,
) -> Result<AppResponse<DeleteVolumeRequest>, ErrorType> {
	trace!("Deleting volume ID: `{volume_id}`");

	query!(
		r#"
		DELETE FROM
			deployment_volume
		WHERE
			id = $1;
		"#,
		volume_id as _
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_foreign_key_violation() => ErrorType::ResourceInUse,
		_ => ErrorType::InternalServerError,
	})?;

	// Mark the resource as deleted in the database
	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		volume_id as _
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?;

	AppResponse::builder()
		.body(DeleteVolumeResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
