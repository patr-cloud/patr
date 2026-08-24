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
				query: DeleteRoleQueryProcessed { remove_users },
				headers: DeleteRoleRequestHeaders {
					authorization: _,
					user_agent: _,
				},
				body: DeleteRoleRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, DeleteRoleRequest>,
) -> Result<AppResponse<DeleteRoleRequest>, ErrorType> {
	info!("Deleting role: {} in workspace: {}", role_id, workspace_id);

	// Seeded default roles are read-only.
	let is_immutable = query!(
		r#"
		SELECT
			is_immutable
		FROM
			role
		WHERE
			id = $1 AND
			workspace_id = $2;
		"#,
		role_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::RoleDoesNotExist)?
	.is_immutable;

	if is_immutable {
		return Err(ErrorType::RoleIsImmutable);
	}

	// Only count when the caller might want to abort. With `remove_users=true`
	// we'd delete regardless, so paying for a COUNT round-trip is wasted work.
	// The handler runs in a transaction, so we *could* rely on the DELETE
	// rolling back on Err — but reading "abort if in use" up front is clearer
	// than "delete, then conditionally return Err to undo the delete via the
	// outer rollback."
	if !remove_users {
		let users_with_role = query!(
			r#"
			SELECT
				COUNT(*) AS "count!: i64"
			FROM
				role_binding
			WHERE
				workspace_id = $1 AND
				role_id = $2;
			"#,
			workspace_id as _,
			role_id as _,
		)
		.fetch_one(&mut **database)
		.await?
		.count;

		if users_with_role > 0 {
			return Err(ErrorType::RoleInUse);
		}
	}

	// Only user bindings FK the role; membership rows are untouched — deleting
	// a role no longer evicts anyone from the workspace. Token ceilings carry
	// permissions rather than roles, so they survive; a token's effective
	// access still shrinks, because the owner's bindings just went.
	let grants_removed = query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	info!("Removed {} binding(s) of the role", grants_removed);

	// Pending invites reference the role too, and the FK won't let it go first.
	query!(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			workspace_id = $1 AND
			role_id = $2;
		"#,
		workspace_id as _,
		role_id as _,
	)
	.execute(&mut **database)
	.await?;

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
			role_permission
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	let role_rows_deleted = query!(
		r#"
		DELETE FROM
			role
		WHERE
			id = $1 AND
			workspace_id = $2;
		"#,
		role_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	if role_rows_deleted == 0 {
		return Err(ErrorType::RoleDoesNotExist);
	}

	// The role is itself a registered resource; tombstone it like every
	// other deleted resource (a hard DELETE would trip audit-log FKs).
	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		role_id as _,
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
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	trace!("Revocation timestamp set");

	AppResponse::builder()
		.body(DeleteRoleResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
