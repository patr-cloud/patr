use crate::prelude::*;

/// Gives every `workspace_user_invite_role` row a scope, by the same three
/// rules as the previous migration's `fill_role_bindings`, then finalises the
/// column.
///
/// The steps run in strict order and each one only touches `scope_id IS NULL`
/// rows, so an earlier rule's output is never re-expanded by a later one. The
/// last step is a catch-all, which is what makes the closing `SET NOT NULL`
/// safe: no row can still be missing a scope by then.
pub(super) async fn fill_user_invite_scope(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	// 1. Workspace scope for Exclude(∅) roles.
	sqlx::query(
		r#"
		UPDATE
			workspace_user_invite_role ir
		SET
			scope_id = ir.workspace_id
		WHERE
			ir.scope_id IS NULL AND EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_type t
				WHERE
					t.role_id = ir.role_id AND
					t.permission_type = 'exclude'
			) AND NOT EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_exclude e
				WHERE
					e.role_id = ir.role_id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 2. Include(S) expansion.
	sqlx::query(
		r#"
		INSERT INTO
			workspace_user_invite_role(
				invite_id,
				workspace_id,
				role_id,
				scope_id
			)
		SELECT
			ir.invite_id,
			ir.workspace_id,
			ir.role_id,
			i.resource_id
		FROM
			workspace_user_invite_role ir
		INNER JOIN
			(
				SELECT DISTINCT
					role_id,
					resource_id
				FROM
					role_resource_permissions_include
			) i
		ON
			i.role_id = ir.role_id
		INNER JOIN
			resource r
		ON
			r.id = i.resource_id AND
			r.workspace_id = ir.workspace_id AND
			r.deleted IS NULL AND
			r.id <> r.workspace_id
		WHERE
			ir.scope_id IS NULL
		ON CONFLICT
			(invite_id, role_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 3. Exclude(S≠∅) expansion.
	sqlx::query(
		r#"
		INSERT INTO
			workspace_user_invite_role(
				invite_id,
				workspace_id,
				role_id,
				scope_id
			)
		SELECT
			ir.invite_id,
			ir.workspace_id,
			ir.role_id,
			r.id
		FROM
			workspace_user_invite_role ir
		INNER JOIN
			resource r
		ON
			r.workspace_id = ir.workspace_id AND
			r.deleted IS NULL AND
			r.id <> r.workspace_id
		WHERE
			ir.scope_id IS NULL AND EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_exclude e
				WHERE
					e.role_id = ir.role_id
			) AND NOT EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_exclude e2
				WHERE
					e2.role_id = ir.role_id AND
					e2.resource_id = r.id
			)
		ON CONFLICT
			(invite_id, role_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 4. Drop the expanded NULL originals. Workspace-scope rows were UPDATEd
	// in step 1, so they no longer match. An include-only invite role whose
	// entire list is dead loses its row here — its grant was empty anyway,
	// and membership-on-accept becomes unconditional at cutover.
	sqlx::query(
		r#"
		DELETE FROM
			workspace_user_invite_role ir
		WHERE
			ir.scope_id IS NULL AND EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_type t
				WHERE
					t.role_id = ir.role_id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 5. Zero-permission roles have no scope to migrate, and a root one would
	// grant everything the moment the role gains a permission. Drop the row,
	// as m020 does. Also the catch-all that leaves nothing NULL below.
	sqlx::query(
		r#"
		DELETE FROM
			workspace_user_invite_role ir
		WHERE
			scope_id IS NULL AND NOT EXISTS (
				SELECT
					1
				FROM
					role_resource_permissions_type
				WHERE
					role_resource_permissions_type.role_id = ir.role_id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Every row now has a scope, so the column can carry its constraint and
	// the primary key can come back with the scope in it.
	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ALTER COLUMN scope_id SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		DROP CONSTRAINT workspace_user_invite_role_uq_invite_id_role_id_scope_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD CONSTRAINT workspace_user_invite_role_pk
		PRIMARY KEY(invite_id, role_id, scope_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
