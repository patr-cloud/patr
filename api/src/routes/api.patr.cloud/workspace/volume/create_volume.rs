use axum::http::StatusCode;
use models::api::workspace::volume::*;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn create_volume(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateVolumePath { workspace_id },
				query: (),
				headers:
					CreateVolumeRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: CreateVolumeRequestProcessed { name, size },
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, CreateVolumeRequest>,
) -> Result<AppResponse<CreateVolumeRequest>, ErrorType> {
	trace!("Creating volume with name: {name}");

	let now = OffsetDateTime::now_utc();
	let volume_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created,
				deleted
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'volume'),
				$1,
				$2,
				NULL
			)
		RETURNING id;
		"#,
		workspace_id as _,
		now
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	query!(
		r#"
		INSERT INTO
			deployment_volume(
				id,
				name,
				volume_size,
				deleted
			)
		VALUES
			(
				$1,
				$2,
				$3,
				NULL
			);
		"#,
		volume_id as _,
		&name,
		size as i32
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(CreateVolumeResponse {
			id: WithId::from(volume_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
