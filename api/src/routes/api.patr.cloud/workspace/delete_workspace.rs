use std::ops::Add;

use axum::http::StatusCode;
use models::api::workspace::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn delete_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteWorkspacePath { workspace_id },
				query: (),
				headers:
					DeleteWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteWorkspaceRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteWorkspaceRequest>,
) -> Result<AppResponse<DeleteWorkspaceRequest>, ErrorType> {
	info!("Deleting workspace `{workspace_id}`");

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

	// Make sure the workspace is owned by the user
	if workspace.super_admin_id != user_data.id.into() {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	// Make sure there are no resources in the workspace
	let resources = query!(
		r#"
		SELECT
			COALESCE(COUNT(*), 0) AS count
		FROM
			resource
		WHERE
			owner_id = $1 AND
			deleted IS NULL;
		"#,
		&workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.and_then(|row| row.count)
	.unwrap_or(0);

	if resources > 0 {
		return Err(ErrorType::WorkspaceNotEmpty);
	}

	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		&workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Revoke all tokens that have access to the workspace
	_ = redis
		.setex(
			redis::keys::workspace_id_revocation_timestamp(&workspace.id.into()),
			constants::ACCESS_TOKEN_VALIDITY.whole_seconds() as u64 + 300,
			OffsetDateTime::now_utc().unix_timestamp(),
		)
		.await;

	AppResponse::builder()
		.body(DeleteWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
