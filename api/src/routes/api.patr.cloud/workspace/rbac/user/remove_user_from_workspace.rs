use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to remove a user from a workspace. This will remove the user
/// from the workspace, and set the revocation timestamp in Redis.
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
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, RemoveUserFromWorkspaceRequest>,
) -> Result<AppResponse<RemoveUserFromWorkspaceRequest>, ErrorType> {
	info!("Removing user `{user_id}` from workspace `{workspace_id}`");

	let rows_removed = query!(
		r#"
		DELETE FROM
			workspace_member
		WHERE
			workspace_id = $1 AND
			identity_id = $2;
		"#,
		workspace_id as _,
		user_id as _
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	if rows_removed == 0 {
		return Err(ErrorType::UserNotFound);
	}

	info!("User removed. Setting revocation timestamp");

	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&user_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	AppResponse::builder()
		.body(RemoveUserFromWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
