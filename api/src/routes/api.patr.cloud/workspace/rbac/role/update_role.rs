use std::collections::BTreeSet;

use axum::http::StatusCode;
use models::{
	api::workspace::rbac::role::*,
	rbac::{PermissionScope, ResourcePermissionType, ResourcePermissionTypeDiscriminant},
};
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to update a role in a workspace. This will update the name,
/// description, and permissions of the role.
pub async fn update_role(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateRolePath {
					role_id,
					workspace_id,
				},
				query: (),
				headers: UpdateRoleRequestHeaders {
					authorization: _,
					user_agent: _,
				},
				body:
					UpdateRoleRequestProcessed {
						role: RoleProcessed { name, description },
						permissions,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateRoleRequest>,
) -> Result<AppResponse<UpdateRoleRequest>, ErrorType> {
	info!("Updating role: {}", role_id);

	if permissions.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	// A binding applies the whole role at one scope, so every permission in a
	// role must carry the same resource set. Non-uniform roles became
	// unrepresentable at the role-binding cutover.
	let mut values = permissions.values();
	let first = values.next();
	if values.any(|value| Some(value) != first) {
		return Err(ErrorType::WrongParameters);
	}

	let rows_updated = query!(
		r#"
		UPDATE
			role
		SET
			name = $1,
			description = $2
		WHERE
			id = $3 AND
			workspace_id = $4;
		"#,
		&*name,
		&*description,
		role_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::RoleAlreadyExists,
		err => ErrorType::server_error(err),
	})?
	.rows_affected();

	if rows_updated == 0 {
		return Err(ErrorType::RoleDoesNotExist);
	}

	trace!("Role updated");

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

	trace!("Role permissions deleted");

	for (permission_id, permission) in permissions {
		let permission_type = ResourcePermissionTypeDiscriminant::from(&permission);
		query!(
			r#"
				INSERT INTO
					role_resource_permissions_type(
						role_id,
						permission_id,
						permission_type
					)
				VALUES
					(
						$1,
						$2,
						$3
					);
				"#,
			role_id as _,
			permission_id as _,
			permission_type as _,
		)
		.execute(&mut **database)
		.await?;
		match permission {
			ResourcePermissionType::Include(resources) => {
				query!(
					r#"
						INSERT INTO
							role_resource_permissions_include(
								role_id,
								permission_id,
								resource_id,
								permission_type
							)
						VALUES
							(
								$1,
								$2,
								UNNEST($3::UUID[]),
								DEFAULT
							);
						"#,
					role_id as _,
					permission_id as _,
					&resources.into_iter().map(|r| r.into()).collect::<Vec<_>>(),
				)
				.execute(&mut **database)
				.await
				.map_err(|err| match err {
					sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
						ErrorType::ResourceDoesNotExist
					}
					other => ErrorType::server_error(other),
				})?;
			}
			ResourcePermissionType::Exclude(resources) => {
				query!(
					r#"
						INSERT INTO
							role_resource_permissions_exclude(
								role_id,
								permission_id,
								resource_id,
								permission_type
							)
						VALUES
							(
								$1,
								$2,
								UNNEST($3::UUID[]),
								DEFAULT
							);
						"#,
					role_id as _,
					permission_id as _,
					&resources.into_iter().map(|r| r.into()).collect::<Vec<_>>(),
				)
				.execute(&mut **database)
				.await
				.map_err(|err| match err {
					sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
						ErrorType::ResourceDoesNotExist
					}
					other => ErrorType::server_error(other),
				})?;
			}
		};
	}

	trace!("Role permissions inserted");

	query!(
		r#"
		DELETE FROM
			role_permission
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_permission(role_id, permission_id)
		SELECT
			$1,
			permission_id
		FROM
			role_resource_permissions_type
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Re-mint the bindings of everyone holding this role at the new scopes.
	// Nothing references binding ids, so delete-and-re-mint has no side
	// effects; token ceilings reference the role directly and are untouched.
	let actor_ids = query!(
		r#"
		SELECT DISTINCT
			actor_id AS "actor_id: Uuid"
		FROM
			role_binding
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| row.actor_id)
	.collect::<Vec<_>>();

	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.execute(&mut **database)
	.await?;

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
		&role_id as _,
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
		&role_id as _,
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
				&role_id as _,
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

	for actor_id in &actor_ids {
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
			actor_id as _,
			&role_id as _,
			&scope_ids as _,
			&user_data.id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	trace!("Bindings re-minted for {} actor(s)", actor_ids.len());

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
		.body(UpdateRoleResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
