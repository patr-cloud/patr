use axum::http::StatusCode;
use models::api::workspace::volume::*;

use crate::prelude::*;

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
		config,
		user_data,
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
		config: config.clone(),
		user_data: user_data.clone(),
	})
	.await?
	.body
	.volume;

	if let Some(size) = size {
		if volume.size > size {
			return Err(ErrorType::CannotReduceVolumeSize);
		}
	}

	query!(
		r#"
		UPDATE
			deployment_volume
		SET
			volume_size = COALESCE($1, volume_size),
			name = COALESCE($2, name)
		WHERE
			id = $3;
		"#,
		size.map(|size| size as i64),
		name.as_deref(),
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
