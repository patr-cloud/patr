//! Helpers for reading and writing `role_binding` rows — the single place a
//! permission target appears after the role-binding cutover.

use std::collections::BTreeSet;

use crate::prelude::*;

/// The scopes a role grant applies at. Until the assignment DTOs carry
/// scopes on the wire, this is read from the legacy per-permission template
/// tables, which the role CRUD handlers keep dual-writing (uniform across a
/// role's permissions — enforced at role write time).
pub enum RoleScopes {
	/// The whole workspace, including resources created later
	/// (`scope_id = workspace_id`).
	Workspace,
	/// Exactly these resources.
	Resources(BTreeSet<Uuid>),
}

/// Ensures an `actor` row exists for the `(user, workspace)` pair, returning
/// its id. The caller must have inserted the membership row first — the FK
/// chain is `role_binding → actor → workspace_user`.
pub async fn ensure_actor_for_user(
	connection: &mut DatabaseConnection,
	user_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<Uuid, sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			actor(id, workspace_id, actor_type, user_id, service_account_id)
		VALUES
			(gen_random_uuid(), $1, 'user', $2, NULL)
		ON CONFLICT
			(user_id, workspace_id)
		DO UPDATE SET
			user_id = EXCLUDED.user_id
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
		user_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.id)
}

/// Reads a role's scope template from the legacy tables, expanded to
/// concrete resources. Returns [`None`] when the role has no permission
/// rows in the given workspace — the caller treats that as
/// role-does-not-exist, since every real role carries at least one
/// permission.
pub async fn read_role_scopes(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	role_id: &Uuid,
) -> Result<Option<RoleScopes>, sqlx::Error> {
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
	.fetch_optional(&mut *connection)
	.await?
	.is_some();

	if !role_exists {
		return Ok(None);
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
	.fetch_optional(&mut *connection)
	.await?
	.is_some();

	if is_workspace_wide {
		return Ok(Some(RoleScopes::Workspace));
	}

	// Include lists name resources directly; Exclude(S≠∅) expands to the
	// live workspace resources not in S. The workspace's own resource row
	// is never a scope — `scope_id = workspace_id` means workspace-wide.
	let resources = query!(
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
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.resource_id)
	.collect::<BTreeSet<_>>();

	Ok(Some(RoleScopes::Resources(resources)))
}

/// Mints bindings for one `(actor, role)` at the given scopes.
pub async fn mint_bindings(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	actor_id: &Uuid,
	role_id: &Uuid,
	scopes: &RoleScopes,
	created_by: Option<&Uuid>,
) -> Result<(), sqlx::Error> {
	let scope_ids = match scopes {
		RoleScopes::Workspace => vec![*workspace_id],
		RoleScopes::Resources(resources) => resources.iter().copied().collect(),
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
		actor_id as _,
		role_id as _,
		&scope_ids as _,
		created_by as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Deletes every binding held by an actor, returning the number removed.
pub async fn delete_bindings_for_actor(
	connection: &mut DatabaseConnection,
	actor_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			actor_id = $1;
		"#,
		actor_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|result| result.rows_affected())
}

/// Deletes every binding of a role — including token ceilings referencing
/// it, which FK the role directly — returning the number of user bindings
/// removed.
pub async fn delete_bindings_for_role(
	connection: &mut DatabaseConnection,
	role_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			api_token_role_binding
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|result| result.rows_affected())
}
