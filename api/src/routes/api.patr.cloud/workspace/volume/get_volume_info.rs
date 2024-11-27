use axum::http::StatusCode;
use models::api::workspace::volume::*;

use crate::prelude::*;

pub async fn get_volume_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetVolumeInfoPath {
					workspace_id: _,
					volume_id,
				},
				query: (),
				headers:
					GetVolumeInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetVolumeInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetVolumeInfoRequest>,
) -> Result<AppResponse<GetVolumeInfoRequest>, ErrorType> {
	trace!("Getting volume info: {}", volume_id);

	let row = query!(
		r#"
		SELECT
			deployment_volume.*,
			deployment_volume_mount.deployment_id AS "deployment_id?: Uuid"
		FROM
			deployment_volume
		LEFT JOIN
			deployment_volume_mount
		ON
			deployment_volume.id = deployment_volume_mount.deployment_id
		WHERE
			deployment_volume.id = $1;
		"#,
		volume_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetVolumeInfoResponse {
			volume: WithId::new(
				row.id,
				DeploymentVolume {
					name: row.name,
					size: row.volume_size.unsigned_abs().into(),
					deployment_id: row.deployment_id,
				},
			),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
