use axum::http::StatusCode;
use models::api::workspace::*;

use crate::{models::permissions, prelude::*};

/// The handler to delete a workspace. This will delete all associated data
/// with the workspace, including the database, container registry, and any
/// other resources. This is a destructive operation and cannot be undone.
/// The workspace must be empty before it can be deleted.
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
		user_data,
		state: _,
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
	if workspace.super_admin_id != user_data.id {
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
			workspace_id = $1 AND
			id <> workspace_id AND
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

	query!(
		r#"
		DELETE FROM
			workspace
		WHERE
			id = $1;
		"#,
		&workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Cached permissions on this workspace are now stale
	permissions::mark_workspace_stale(redis, &workspace.id.into()).await?;

	AppResponse::builder()
		.body(DeleteWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
