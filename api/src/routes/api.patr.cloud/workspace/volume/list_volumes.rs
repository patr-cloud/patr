use axum::http::StatusCode;
use models::api::workspace::volume::*;

use crate::prelude::*;

pub async fn list_volumes(
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
			id: WithId::new(workspace_id, ()),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
