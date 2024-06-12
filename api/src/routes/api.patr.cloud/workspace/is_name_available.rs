use axum::http::StatusCode;
use models::api::workspace::*;

use crate::prelude::*;

/// The handler to check if a workspace name is available. This is used when
/// creating a new workspace to ensure that the name is unique.
pub async fn is_name_available(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: IsWorkspaceNameAvailablePath,
				query: IsWorkspaceNameAvailableQuery { name },
				headers:
					IsWorkspaceNameAvailableRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: IsWorkspaceNameAvailableRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, IsWorkspaceNameAvailableRequest>,
) -> Result<AppResponse<IsWorkspaceNameAvailableRequest>, ErrorType> {
	info!("Checking if workspace name `{name}` is available");

	let available = query!(
		r#"
        SELECT
            *
        FROM
            workspace
        WHERE
            name = $1;
        "#,
		&name,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_none();

	AppResponse::builder()
		.body(IsWorkspaceNameAvailableResponse { available })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
