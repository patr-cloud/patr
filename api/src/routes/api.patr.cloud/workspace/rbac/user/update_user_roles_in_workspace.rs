use std::collections::BTreeSet;

use axum::http::StatusCode;
use models::{api::workspace::rbac::user::*, rbac::PermissionScope};
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

	// Membership is unconditional and independent of role-holding: an empty
	// roles list drops the user's bindings but keeps them a member. Removal
	// from the workspace is RemoveUserFromWorkspace's job.
	query!(
		r#"
		INSERT INTO
			workspace_user(user_id, workspace_id)
		VALUES
			($1, $2)
		ON CONFLICT
			(user_id, workspace_id)
		DO NOTHING;
		"#,
		user_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			ErrorType::UserNotFound
		}
		other => ErrorType::server_error(other),
	})?;

	let actor_id = query!(
		r#"
		INSERT INTO
			workspace_actor(id, workspace_id, actor_type, user_id)
		VALUES
			(gen_random_uuid(), $1, 'user', $2)
		ON CONFLICT
			(user_id, workspace_id)
		DO UPDATE SET
			user_id = EXCLUDED.user_id
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
		&user_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			actor_id = $1;
		"#,
		&actor_id as _,
	)
	.execute(&mut **database)
	.await?;

	for role_id in &roles {
		let role_exists = query!(
			r#"
			SELECT
				1 AS "present"
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
		.is_some();

		if !role_exists {
			return Err(ErrorType::RoleDoesNotExist);
		}

		// Uniformity is enforced at role write time, so one permission's shape
		// speaks for the whole role. Exclude with no children = workspace-wide.
		let is_workspace_wide = query!(
			r#"
			SELECT
				1 AS "present"
			FROM
				role_resource_permissions_type t
			WHERE
				t.role_id = $1 AND
				t.permission_type = 'exclude' AND
				NOT EXISTS (
					SELECT
						1
					FROM
						role_resource_permissions_exclude e
					WHERE
						e.role_id = t.role_id
				);
			"#,
			role_id as _,
		)
		.fetch_optional(&mut **database)
		.await?
		.is_some();

		// Include lists name resources directly; Exclude(S≠∅) expands to the live
		// workspace resources not in S. The workspace's own resource row is never
		// a scope — `scope_id = workspace_id` means workspace-wide.
		let scopes = if is_workspace_wide {
			PermissionScope::Workspace
		} else {
			PermissionScope::Resources(
				query!(
					r#"
					SELECT
						i.resource_id AS "resource_id!: Uuid"
					FROM
						(SELECT DISTINCT resource_id FROM role_resource_permissions_include WHERE role_id = $1) i
					INNER JOIN
						resource r
					ON
						r.id = i.resource_id AND
						r.workspace_id = $2 AND
						r.deleted IS NULL AND
						r.id <> r.workspace_id
					UNION
					SELECT
						r.id
					FROM
						resource r
					WHERE
						r.workspace_id = $2 AND
						r.deleted IS NULL AND
						r.id <> r.workspace_id AND
						EXISTS (
							SELECT 1 FROM role_resource_permissions_exclude e WHERE e.role_id = $1
						) AND
						NOT EXISTS (
							SELECT
								1
							FROM
								role_resource_permissions_exclude e
							WHERE
								e.role_id = $1 AND
								e.resource_id = r.id
						);
					"#,
					role_id as _,
					workspace_id as _,
				)
				.fetch_all(&mut **database)
				.await?
				.into_iter()
				.map(|row| row.resource_id)
				.collect::<BTreeSet<_>>(),
			)
		};
		let scope_ids = match &scopes {
			PermissionScope::Workspace => vec![workspace_id],
			PermissionScope::Resources(resources) => resources.iter().copied().collect(),
		};

		query!(
			r#"
			INSERT INTO
				role_binding(id, workspace_id, actor_id, role_id, scope_id, created, created_by)
			SELECT
				gen_random_uuid(),
				$1,
				$2,
				$3,
				scope_id,
				NOW(),
				$5
			FROM
				UNNEST($4::UUID[]) AS scopes(scope_id)
			ON CONFLICT
				(actor_id, role_id, scope_id)
			DO NOTHING;
			"#,
			workspace_id as _,
			&actor_id as _,
			role_id as _,
			&scope_ids as _,
			&user_data.id as _,
		)
		.execute(&mut **database)
		.await?;
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
