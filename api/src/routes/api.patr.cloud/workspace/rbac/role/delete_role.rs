use axum::http::StatusCode;
use models::api::workspace::rbac::role::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// Deletes a role from the workspace and revokes the cached permissions. This
/// will delete all the permissions associated with the role. Any user that has
/// the role will have it removed, if the `remove_users` query parameter is set
/// to true. Otherwise, an error will be thrown.
pub async fn delete_role(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteRolePath {
					workspace_id,
					role_id,
				},
				query: DeleteRoleQuery { remove_users },
				headers: DeleteRoleRequestHeaders {
					authorization: _,
					user_agent: _,
				},
				body: DeleteRoleRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteRoleRequest>,
) -> Result<AppResponse<DeleteRoleRequest>, ErrorType> {
	info!("Deleting role: {} in workspace: {}", role_id, workspace_id);

	// Remove the role from all the users. If the role is still in use, an error
	// will be thrown, causing the transaction to be rolled back and the role not
	// to be deleted
	let users_with_role = query!(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			workspace_id = $1 AND
			role_id = $2;
		"#,
		workspace_id as _,
		role_id as _
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	info!("Removed role from {} users", users_with_role);

	if !remove_users && users_with_role > 0 {
		// The role is still in use
		return Err(ErrorType::RoleInUse);
	}

	query!(
		r#"
        DELETE FROM
            role_resource_permissions_include
        WHERE
            role_id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the included permissions");

	query!(
		r#"
        DELETE FROM
            role_resource_permissions_exclude
        WHERE
            role_id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the excluded permissions");

	query!(
		r#"
        DELETE FROM
            role_resource_permissions_type
        WHERE
            role_id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the permission types");

	query!(
		r#"
        DELETE FROM
            role
        WHERE
            id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted the role");

	redis
		.setex(
			redis::keys::workspace_id_revocation_timestamp(&workspace_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	trace!("Revocation timestamp set");

	AppResponse::builder()
		.body(DeleteRoleResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
