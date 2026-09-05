use std::collections::BTreeSet;

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
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateUserRolesInWorkspaceRequest>,
) -> Result<AppResponse<UpdateUserRolesInWorkspaceRequest>, ErrorType> {
	info!("Updating user `{user_id}`'s roles in workspace `{workspace_id}`");

	let roles = roles.into_iter().collect::<BTreeSet<_>>();

	// Only an existing member's roles can be updated; adding someone to the
	// workspace is InviteUserToWorkspace's job.
	query!(
		r#"
		SELECT
			1 AS "present"
		FROM
			workspace_user
		WHERE
			user_id = $1 AND
			workspace_id = $2;
		"#,
		user_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	let actor_id = query!(
		r#"
		SELECT
			actor_id AS "id: Uuid"
		FROM
			workspace_user
		WHERE
			user_id = $1 AND
			workspace_id = $2;
		"#,
		&user_id as _,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	// The exact set of bindings this actor should end up with. Rows that
	// survive the diff keep their original id and attribution.
	let (role_ids, scope_ids) = roles
		.iter()
		.map(|grant| (grant.role_id, grant.resource_id))
		.collect::<(Vec<_>, Vec<_>)>();

	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			actor_id = $1 AND
			(role_id, scope_id) NOT IN (
				SELECT
					role_id,
					scope_id
				FROM
					UNNEST($2::UUID[], $3::UUID[]) AS requested(role_id, scope_id)
			);
		"#,
		&actor_id as _,
		&role_ids as _,
		&scope_ids as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_binding(
				id,
				workspace_id,
				actor_id,
				role_id,
				scope_id,
				created,
				created_by
			)
		SELECT
			gen_random_uuid(),
			$1,
			$2,
			requested.role_id,
			requested.scope_id,
			NOW(),
			$5
		FROM
			UNNEST($3::UUID[], $4::UUID[]) AS requested(role_id, scope_id)
		ON CONFLICT
			(actor_id, role_id, scope_id)
		DO NOTHING;
		"#,
		workspace_id as _,
		&actor_id as _,
		&role_ids as _,
		&scope_ids as _,
		&user_data.id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			match db_err.constraint() {
				Some("role_binding_fk_role_id_workspace_id") => ErrorType::RoleDoesNotExist,
				Some("role_binding_fk_scope_id_workspace_id") => ErrorType::ResourceDoesNotExist,
				_ => ErrorType::server_error(sqlx::Error::Database(db_err)),
			}
		}
		other => ErrorType::server_error(other),
	})?;

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
