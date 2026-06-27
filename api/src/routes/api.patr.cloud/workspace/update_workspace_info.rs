use axum::http::StatusCode;
use models::api::workspace::*;

use crate::prelude::*;

/// The handler to update the information of a workspace. At the moment, only
/// the name can be updated. However, this will be expanded in the future.
pub async fn update_workspace_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateWorkspaceInfoPath { workspace_id },
				query: (),
				headers:
					UpdateWorkspaceInfoRequestHeaders {
						authorization,
						user_agent,
					},
				body: UpdateWorkspaceInfoRequestProcessed { name },
			},
		database,
		redis,
		client_ip,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, UpdateWorkspaceInfoRequest>,
) -> Result<AppResponse<UpdateWorkspaceInfoRequest>, ErrorType> {
	info!("Updating information for workspace `{workspace_id}`");

	let current_name = query!(
		r#"
		SELECT
			name
		FROM
			workspace
		WHERE
			id = $1;
		"#,
		&workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?
	.name;

	// The full object always carries the name, so only check availability when it
	// actually changed — otherwise the workspace's own name would fail the check.
	if *name != *current_name {
		let available = super::is_name_available(AuthenticatedAppRequest {
			request: ProcessedApiRequest {
				path: IsWorkspaceNameAvailablePath,
				query: IsWorkspaceNameAvailableQueryProcessed {
					name: name.to_string().into(),
				},
				headers: IsWorkspaceNameAvailableRequestHeaders {
					authorization,
					user_agent,
				},
				body: IsWorkspaceNameAvailableRequestProcessed,
			},
			database,
			redis,
			client_ip,
			user_data,
			state,
		})
		.await?
		.body
		.available;

		if !available {
			return Err(ErrorType::WorkspaceNameAlreadyExists);
		}
	}

	query!(
		r#"
		UPDATE
			workspace
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		&*name,
		&workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(UpdateWorkspaceInfoResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
