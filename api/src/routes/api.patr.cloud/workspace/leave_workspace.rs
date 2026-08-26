use axum::http::StatusCode;
use models::{api::workspace::*, rbac::WorkspacePermission};
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler for the authenticated user to leave a workspace. The owner
/// (super admin) of a workspace cannot leave it — they must transfer or delete
/// it instead.
pub async fn leave_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: LeaveWorkspacePath { workspace_id },
				query: (),
				headers:
					LeaveWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: LeaveWorkspaceRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, LeaveWorkspaceRequest>,
) -> Result<AppResponse<LeaveWorkspaceRequest>, ErrorType> {
	info!("User `{}` leaving workspace `{workspace_id}`", user_data.id);

	if user_data
		.permissions
		.get(&workspace_id)
		.is_some_and(WorkspacePermission::is_super_admin)
	{
		return Err(ErrorType::CannotLeaveWorkspaceAsOwner);
	}

	// Ordered teardown: bindings -> actor -> membership.
	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			actor_id IN (
				SELECT
					id
				FROM
					workspace_actor
				WHERE
					user_id = $1 AND
					workspace_id = $2
			);
		"#,
		user_data.id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			workspace_actor
		WHERE
			user_id = $1 AND
			workspace_id = $2;
		"#,
		user_data.id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			workspace_id = $1 AND
			user_id = $2;
		"#,
		workspace_id as _,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	info!("User left workspace. Setting revocation timestamp");

	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&user_data.id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{err}`");
		})?;

	AppResponse::builder()
		.body(LeaveWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
