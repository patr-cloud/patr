use axum::http::StatusCode;
use models::api::workspace::volume::*;

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
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, CreateVolumeRequest>,
) -> Result<AppResponse<CreateVolumeRequest>, ErrorType> {
	AppResponse::builder()
		.body(CreateVolumeResponse {
			id: WithId::from(workspace_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
