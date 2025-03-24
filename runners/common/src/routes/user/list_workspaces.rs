use axum::http::StatusCode;
use models::api::{WithId, user::*, workspace::Workspace};

use crate::prelude::*;

/// The handler to list all the workspaces of the user. This will return the
/// default workspace, since this will only be called in self hosted mode and
/// there is no concept of workspaces in self hosted mode.
#[instrument]
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
		change_publisher: _,
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
