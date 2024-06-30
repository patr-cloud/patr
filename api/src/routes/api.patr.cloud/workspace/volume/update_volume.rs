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
				headers:
					UpdateVolumeRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: UpdateVolumeRequestProcessed { name, size },
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, UpdateVolumeRequest>,
) -> Result<AppResponse<UpdateVolumeRequest>, ErrorType> {
	AppResponse::builder()
		.body(UpdateVolumeResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
