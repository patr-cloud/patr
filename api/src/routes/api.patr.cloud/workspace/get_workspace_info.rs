use axum::http::StatusCode;
use models::api::workspace::*;

use crate::prelude::*;

/// The handler to get the information of a workspace. This includes the
/// workspace's name, the user who created it, and the date it was created.
pub async fn get_workspace_info(
	AuthenticatedAppRequest {
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
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetWorkspaceInfoRequest>,
) -> Result<AppResponse<GetWorkspaceInfoRequest>, ErrorType> {
	info!("Getting information about the workspace `{workspace_id}`");

	let workspace = query!(
		r#"
		SELECT
			*
		FROM
			workspace
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		&workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetWorkspaceInfoResponse {
			workspace: WithId::new(
				workspace_id,
				Workspace {
					name: workspace.name,
					super_admin_id: workspace.super_admin_id.into(),
				},
			),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
