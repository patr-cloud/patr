use axum::http::StatusCode;
use models::api::workspace::volume::*;

use crate::prelude::*;

pub async fn delete_volume(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteVolumePath {
					workspace_id,
					volume_id,
				},
				query: (),
				headers:
					DeleteVolumeRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteVolumeRequestProcessed { name, size },
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteVolumeRequest>,
) -> Result<AppResponse<DeleteVolumeRequest>, ErrorType> {
	AppResponse::builder()
		.body(DeleteVolumeResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
