use crate::prelude::*;

/// Adds `scope_id` to `workspace_user_invite_role` and drops the old primary
/// key, so the expansion can put more than one row per `(invite, role)`.
///
/// The column starts nullable only because the value differs per row and has
/// to be computed. [`super::fill_user_invite_scope`] fills every row and then
/// restores both the `NOT NULL` and the primary key — all inside this
/// migration's single transaction, so the duplicate protection the old key
/// gave is never actually absent to anyone.
///
/// A `UNIQUE NULLS NOT DISTINCT` on the eventual key columns stands in for the
/// dropped primary key in the meantime. It is not just belt-and-braces: the
/// expansion inserts with `ON CONFLICT (invite_id, role_id, scope_id)`, and
/// Postgres can only infer that target from an existing unique index. Without
/// it the expansion fails outright. `NULLS NOT DISTINCT` because `scope_id` is
/// still NULL on every pre-existing row, and the default (NULLs distinct)
/// would let the old `(invite, role)` pair duplicate while the key is down.
pub(super) async fn user_invite_add_scope(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD COLUMN scope_id UUID;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		DROP CONSTRAINT workspace_user_invite_role_pk;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD CONSTRAINT workspace_user_invite_role_uq_invite_id_role_id_scope_id
		UNIQUE NULLS NOT DISTINCT (invite_id, role_id, scope_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD CONSTRAINT workspace_user_invite_role_fk_scope_id_workspace_id
		FOREIGN KEY(scope_id, workspace_id) REFERENCES resource(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
