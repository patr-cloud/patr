use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to update a user's roles in a workspace. This requires the user
/// who is sending the request to have the permission to update roles in the
/// workspace.
pub async fn update_user_roles_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateUserRolesInWorkspacePath {
					workspace_id,
					user_id,
				},
				query: (),
				headers:
					UpdateUserRolesInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: UpdateUserRolesInWorkspaceRequestProcessed { roles },
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, UpdateUserRolesInWorkspaceRequest>,
) -> Result<AppResponse<UpdateUserRolesInWorkspaceRequest>, ErrorType> {
	info!("Updating user `{user_id}`'s roles in workspace `{workspace_id}`");

	query!(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			workspace_id = $1 AND
			user_id = $2;
		"#,
		workspace_id as _,
		user_id as _
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			workspace_user(
				workspace_id,
				user_id,
				role_id
			)
		VALUES
			($1, $2, UNNEST($3::UUID[]));
		"#,
		workspace_id as _,
		user_id as _,
		&roles
			.into_iter()
			.map(|role| role.into())
			.collect::<Vec<_>>(),
	)
	.execute(&mut **database)
	.await?;

	info!("User's roles updated. Setting revocation timestamp");

	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&user_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	AppResponse::builder()
		.body(UpdateUserRolesInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
