use axum::http::StatusCode;
use models::api::workspace::volume::*;

use crate::prelude::*;

/// This function updates the volume with the given ID. It will update the
/// volume's name and size. If the size is reduced, it will return an error. It
/// can be used to grow the size of a volume or rename it.
pub async fn update_volume(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateVolumePath {
					workspace_id,
					volume_id,
				},
				query: (),
				headers: UpdateVolumeRequestHeaders {
					authorization,
					user_agent,
				},
				body: UpdateVolumeRequestProcessed { name, size },
			},
		database,
		redis,
		client_ip,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, UpdateVolumeRequest>,
) -> Result<AppResponse<UpdateVolumeRequest>, ErrorType> {
	let volume = super::get_volume_info(AuthenticatedAppRequest {
		request: ProcessedApiRequest::builder()
			.path(GetVolumeInfoPath {
				workspace_id,
				volume_id,
			})
			.query(())
			.headers(GetVolumeInfoRequestHeaders {
				authorization: authorization.clone(),
				user_agent: user_agent.clone(),
			})
			.body(GetVolumeInfoRequestProcessed)
			.build(),
		database,
		redis,
		client_ip,
		user_data,
		state,
	})
	.await?
	.body
	.volume;

	if volume.size > u64::from(size) {
		return Err(ErrorType::CannotReduceVolumeSize);
	}

	query!(
		r#"
		UPDATE
			deployment_volume
		SET
			volume_size = $1,
			name = $2
		WHERE
			id = $3;
		"#,
		i64::from(size),
		&*name,
		volume_id as _
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(UpdateVolumeResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
