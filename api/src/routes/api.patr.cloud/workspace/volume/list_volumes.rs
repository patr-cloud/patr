use axum::http::StatusCode;
use models::{api::workspace::volume::*, utils::TotalCountHeader};

use crate::prelude::*;

pub async fn list_volumes(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListVolumesInWorkspacePath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListVolumesInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListVolumesInWorkspaceRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListVolumesInWorkspaceRequest>,
) -> Result<AppResponse<ListVolumesInWorkspaceRequest>, ErrorType> {
	AppResponse::builder()
		.body(ListVolumesInWorkspaceResponse { volumes: todo!() })
		.headers(ListVolumesInWorkspaceResponseHeaders {
			total_count: TotalCountHeader(todo!() as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
