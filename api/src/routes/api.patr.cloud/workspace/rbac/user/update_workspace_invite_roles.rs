use std::collections::BTreeSet;

use axum::http::StatusCode;
use models::{api::workspace::rbac::user::*, rbac::PermissionScope};

use crate::prelude::*;

/// The handler to update the roles a pending invite will grant once accepted.
/// Requires the permission to modify roles. The invite token and email are left
/// untouched — only the granted role set changes.
pub async fn update_workspace_invite_roles(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateWorkspaceInviteRolesPath {
					workspace_id,
					invite_id,
				},
				query: (),
				headers:
					UpdateWorkspaceInviteRolesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: UpdateWorkspaceInviteRolesRequestProcessed { roles },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateWorkspaceInviteRolesRequest>,
) -> Result<AppResponse<UpdateWorkspaceInviteRolesRequest>, ErrorType> {
	info!("Updating roles for invite `{invite_id}` in workspace `{workspace_id}`");

	let roles = roles.into_iter().collect::<BTreeSet<_>>();

	if roles.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	let exists = query!(
		r#"
		SELECT EXISTS(
			SELECT
				1
			FROM
				workspace_user_invite
			WHERE
				id = $1 AND
				workspace_id = $2
		) AS "exists!: bool";
		"#,
		invite_id as _,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.exists;

	if !exists {
		return Err(ErrorType::InviteNotFound);
	}

	// The request rolls back on failure, so the invite keeps its old roles.
	query!(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			invite_id = $1;
		"#,
		invite_id as _,
	)
	.execute(&mut **database)
	.await?;

	// One row per (role, scope), snapshotting the roles' scopes now — same
	// shape InviteUserToWorkspace writes.
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
		let scope_ids = match scopes {
			PermissionScope::Workspace => vec![workspace_id],
			PermissionScope::Resources(resources) => resources.into_iter().collect(),
		};

		query!(
			r#"
			INSERT INTO
				workspace_user_invite_role(
					invite_id,
					workspace_id,
					role_id,
					scope_id
				)
			SELECT
				$1, $2, $3, *
			FROM
				UNNEST($4::UUID[]);
			"#,
			invite_id as _,
			workspace_id as _,
			role_id as _,
			&scope_ids as _,
		)
		.execute(&mut **database)
		.await
		.map_err(|err| match err {
			sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
				ErrorType::RoleDoesNotExist
			}
			other => ErrorType::server_error(other),
		})?;
	}

	AppResponse::builder()
		.body(UpdateWorkspaceInviteRolesResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
