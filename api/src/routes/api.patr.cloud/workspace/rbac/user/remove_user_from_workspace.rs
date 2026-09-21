use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;

use crate::{models::permissions, prelude::*};

/// The handler to remove a user from a workspace. This will remove the user
/// from the workspace, and mark their cached permissions stale.
pub async fn remove_user_from_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RemoveUserFromWorkspacePath {
					workspace_id,
					user_id,
				},
				query: (),
				headers:
					RemoveUserFromWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RemoveUserFromWorkspaceRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, RemoveUserFromWorkspaceRequest>,
) -> Result<AppResponse<RemoveUserFromWorkspaceRequest>, ErrorType> {
	info!("Removing user `{user_id}` from workspace `{workspace_id}`");

	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			actor_id IN (
				SELECT
					actor_id
				FROM
					workspace_user
				WHERE
					user_id = $1 AND
					workspace_id = $2
			);
		"#,
		user_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	let actor_id = query!(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			workspace_id = $1 AND
			user_id = $2
		RETURNING
			actor_id AS "actor_id: Uuid";
		"#,
		workspace_id as _,
		user_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?
	.actor_id;

	query!(
		r#"
		DELETE FROM
			workspace_actor
		WHERE
			id = $1;
		"#,
		&actor_id as _,
	)
	.execute(&mut **database)
	.await?;

	info!("User removed. Marking cached permissions stale");

	permissions::mark_actor_stale(redis, &user_id).await?;

	AppResponse::builder()
		.body(RemoveUserFromWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
