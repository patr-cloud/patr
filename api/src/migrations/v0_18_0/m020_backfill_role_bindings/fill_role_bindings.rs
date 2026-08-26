use crate::prelude::*;

/// Mints a binding for every role assignment, by the three expansion rules.
///
/// Backfilled bindings are attributed to the workspace's super-admin. The
/// legacy tables record who *holds* a role, never who granted it, so there is
/// no truthful author to carry over — the super-admin is the one account that
/// could have made every one of these grants.
///
/// Post-split every role is uniform — all of its permissions carry the same
/// resource set — so a role can be classified as a whole rather than per
/// permission.
pub(super) async fn fill_role_bindings(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	// Exclude(∅): the whole workspace, one binding at scope = workspace.
	sqlx::query(
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
			wu.workspace_id,
			a.id,
			wu.role_id,
			wu.workspace_id,
			NOW(),
			workspace.super_admin_id
		FROM
			workspace_user wu
		INNER JOIN
			workspace
		ON
			workspace.id = wu.workspace_id
		INNER JOIN
			workspace_actor a
		ON
			a.user_id = wu.user_id AND
			a.workspace_id = wu.workspace_id
		WHERE
			EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_type t
				WHERE
					t.role_id = wu.role_id AND
					t.permission_type = 'exclude'
			) AND NOT EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_exclude e
				WHERE
					e.role_id = wu.role_id
			)
		ON CONFLICT
			(actor_id, role_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Include(S): one binding per live, same-workspace member of S.
	// Cross-workspace include rows are dead grants under the corrected
	// evaluator, and deleted resources cannot come back.
	sqlx::query(
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
			wu.workspace_id,
			a.id,
			wu.role_id,
			i.resource_id,
			NOW(),
			workspace.super_admin_id
		FROM
			workspace_user wu
		INNER JOIN
			workspace
		ON
			workspace.id = wu.workspace_id
		INNER JOIN
			workspace_actor a
		ON
			a.user_id = wu.user_id AND
			a.workspace_id = wu.workspace_id
		INNER JOIN
			(
				SELECT DISTINCT
					role_id,
					resource_id
				FROM
					role_resource_permissions_include
			) i
		ON
			i.role_id = wu.role_id
		INNER JOIN
			resource r
		ON
			r.id = i.resource_id AND
			r.workspace_id = wu.workspace_id AND
			r.deleted IS NULL AND
			r.id <> r.workspace_id
		ON CONFLICT
			(actor_id, role_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Exclude(S≠∅): one binding per live workspace resource not in S.
	sqlx::query(
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
			wu.workspace_id,
			a.id,
			wu.role_id,
			r.id,
			NOW(),
			workspace.super_admin_id
		FROM
			workspace_user wu
		INNER JOIN
			workspace
		ON
			workspace.id = wu.workspace_id
		INNER JOIN
			workspace_actor a
		ON
			a.user_id = wu.user_id AND
			a.workspace_id = wu.workspace_id
		INNER JOIN
			resource r
		ON
			r.workspace_id = wu.workspace_id AND
			r.deleted IS NULL AND
			r.id <> r.workspace_id
		WHERE
			EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_exclude e
				WHERE
					e.role_id = wu.role_id
			) AND
			NOT EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_exclude e2
				WHERE
					e2.role_id = wu.role_id AND
					e2.resource_id = r.id
			)
		ON CONFLICT
			(actor_id, role_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
