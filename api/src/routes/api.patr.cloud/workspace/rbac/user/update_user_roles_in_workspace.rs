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
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateUserRolesInWorkspaceRequest>,
) -> Result<AppResponse<UpdateUserRolesInWorkspaceRequest>, ErrorType> {
	info!("Updating user `{user_id}`'s roles in workspace `{workspace_id}`");

	let expected_role_count = roles.len();

	// When the caller passes an empty roles list, the intent is "remove this
	// user from the workspace". The DELETE below silently no-ops on a
	// non-member; surface UserNotFound explicitly so callers don't think a
	// removal happened when it didn't.
	if roles.is_empty() {
		let is_member = query!(
			r#"
			SELECT EXISTS(
				SELECT
					1
				FROM
					workspace_member
				WHERE
					workspace_id = $1 AND
					identity_id = $2
			) AS "exists!: bool";
			"#,
			workspace_id as _,
			user_id as _,
		)
		.fetch_one(&mut **database)
		.await?
		.exists;

		if !is_member {
			return Err(ErrorType::UserNotFound);
		}
	}

	query!(
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
	.await?;

	// Use a CTE that filters role_ids by workspace ownership before inserting.
	// If any role_id is missing or belongs to a different workspace, fewer rows
	// will be inserted than requested — surface that as RoleDoesNotExist.
	let inserted = query!(
		r#"
		WITH valid_roles AS (
			SELECT
				id
			FROM
				role
			WHERE
				id = ANY($3::UUID[]) AND
				owner_id = $1
		)
		INSERT INTO
			workspace_member(
				workspace_id,
				identity_id,
				role_id
			)
		SELECT
			$1, $2, id
		FROM
			valid_roles;
		"#,
		workspace_id as _,
		user_id as _,
		roles as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			match db_err.constraint() {
				Some(c) if c == "workspace_member_fk_role_id_workspace_id" => {
					ErrorType::RoleDoesNotExist
				}
				Some(c) if c == "workspace_member_fk_identity_id" => ErrorType::UserNotFound,
				_ => ErrorType::server_error(sqlx::Error::Database(db_err)),
			}
		}
		other => ErrorType::server_error(other),
	})?
	.rows_affected() as usize;

	if inserted != expected_role_count {
		return Err(ErrorType::RoleDoesNotExist);
	}

	info!("User's roles updated. Setting revocation timestamp");

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
		.body(UpdateUserRolesInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
