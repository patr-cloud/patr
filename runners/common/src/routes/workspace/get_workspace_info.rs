use axum::http::StatusCode;
use models::api::workspace::*;

use crate::prelude::*;

/// The handler to get the information of a workspace. This includes the
/// workspace's name, the user who created it, and the date it was created.
pub async fn get_workspace_info(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GetWorkspaceInfoPath { workspace_id },
				query: (),
				headers:
					GetWorkspaceInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetWorkspaceInfoRequestProcessed,
			},
		database,
		config: _,
	}: AppRequest<'_, GetWorkspaceInfoRequest>,
) -> Result<AppResponse<GetWorkspaceInfoRequest>, ErrorType> {
	info!("Getting information about the workspace `{workspace_id}`");

	if !workspace_id.is_nil() {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	AppResponse::builder()
		.body(GetWorkspaceInfoResponse {
			workspace: WithId::new(
				Uuid::nil(),
				Workspace {
					name: "Default".to_string(),
					super_admin_id: Uuid::nil(),
				},
			),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
