use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
use models::{api::workspace::rbac::user::*, rbac::WorkspacePermission};

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
		actor_data,
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
		invite_id,
		workspace_id,
	)
	.fetch_one(&mut **database)
	.await?
	.exists;

	if !exists {
		return Err(ErrorType::InviteNotFound);
	}

	// One row per grant, straight from the request.
	let role_ids = roles.iter().map(|grant| grant.role_id).collect::<Vec<_>>();
	let scope_ids = roles
		.iter()
		.map(|grant| grant.resource_id)
		.collect::<Vec<_>>();

	// Diff against what is requested rather than wipe and reinsert.
	// The request rolls back on failure, so the invite keeps its old roles.
	query!(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			invite_id = $1 AND
			(role_id, scope_id) NOT IN (
				SELECT
					role_id,
					scope_id
				FROM
					UNNEST($2::UUID[], $3::UUID[]) AS requested(role_id, scope_id)
			);
		"#,
		invite_id,
		&role_ids as _,
		&scope_ids as _,
	)
	.execute(&mut **database)
	.await?;

	// Joined to the permissions these bindings grant, to check the caller holds them.
	let granted = query!(
		r#"
		WITH inserted AS (
			INSERT INTO
				workspace_user_invite_role(
					invite_id,
					workspace_id,
					role_id,
					scope_id
				)
			SELECT
				$1,
				$2,
				UNNEST($3::UUID[]),
				UNNEST($4::UUID[])
			ON CONFLICT
				(invite_id, role_id, scope_id)
			DO NOTHING
			RETURNING
				role_id,
				scope_id
		)
		SELECT
			role_permission.permission_id AS "permission_id!",
			inserted.scope_id AS "scope_id!"
		FROM
			inserted
		INNER JOIN
			role_permission
		ON
			role_permission.role_id = inserted.role_id;
		"#,
		invite_id,
		workspace_id,
		&role_ids as _,
		&scope_ids as _,
	)
	.fetch_all(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			match db_err.constraint() {
				Some("workspace_user_invite_role_fk_scope_id_workspace_id") => {
					ErrorType::ResourceDoesNotExist
				}
				_ => ErrorType::RoleDoesNotExist,
			}
		}
		other => ErrorType::server_error(other),
	})?;

	if !actor_data
		.permissions
		.get(&workspace_id)
		.is_some_and(|held| {
			held.is_superset_of(
				&WorkspacePermission::Member {
					permissions: granted
						.iter()
						.fold(BTreeMap::new(), |mut permissions, row| {
							permissions
								.entry(row.permission_id)
								.or_insert_with(BTreeSet::new)
								.insert(row.scope_id);
							permissions
						}),
				},
				workspace_id,
			)
		}) {
		return Err(ErrorType::GrantExceedsPermissions);
	}

	AppResponse::builder()
		.body(UpdateWorkspaceInviteRolesResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
