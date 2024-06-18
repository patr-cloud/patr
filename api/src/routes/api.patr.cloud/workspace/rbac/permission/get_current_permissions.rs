use axum::http::StatusCode;
use models::api::workspace::rbac::*;

use crate::prelude::*;

/// The handler to get the permissions of the current request. This will return
/// the permissions of the currently authenticated user in the workspace.
pub async fn get_current_permissions(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetCurrentPermissionsPath { workspace_id },
				query: (),
				headers:
					GetCurrentPermissionsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetCurrentPermissionsRequestProcessed,
			},
		database: _,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetCurrentPermissionsRequest>,
) -> Result<AppResponse<GetCurrentPermissionsRequest>, ErrorType> {
	info!("Get permissions of current request");

	AppResponse::builder()
		.body(GetCurrentPermissionsResponse {
			permissions: user_data
				.permissions
				.get(&workspace_id)
				.ok_or(ErrorType::WrongParameters)?
				.clone(),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
