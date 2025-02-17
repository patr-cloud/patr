use axum::http::StatusCode;
use models::api::{user::*, workspace::Workspace, WithId};

use crate::prelude::*;

pub async fn list_workspaces(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ListUserWorkspacesPath,
				query: (),
				headers:
					ListUserWorkspacesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListUserWorkspacesRequestProcessed,
			},
		database: _,
		config: _,
	}: AppRequest<'_, ListUserWorkspacesRequest>,
) -> Result<AppResponse<ListUserWorkspacesRequest>, ErrorType> {
	info!("Listing all user workspaces");

	let workspaces = vec![WithId::new(
		Uuid::nil(),
		Workspace {
			name: "Default".into(),
			super_admin_id: Uuid::nil(),
		},
	)];

	AppResponse::builder()
		.body(ListUserWorkspacesResponse { workspaces })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
