//! Ties a role grant to the workspace it was granted in, at the schema level.
//!
//! `workspace_user` carried `workspace_id` and `role_id` as two independent
//! foreign keys, so nothing stopped a row from granting a user a role that
//! belongs to a *different* workspace. Giving `role` a `UNIQUE(id, owner_id)`
//! lets the membership row reference the pair instead, which makes that state
//! unrepresentable. The same key is what `workspace_user_invite_role` points
//! at, so pending invites get the guarantee too.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE role
		ADD CONSTRAINT role_uq_id_owner_id
		UNIQUE(id, owner_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// The composite key below is validated against existing rows, so any grant
	// that already crosses workspaces has to go first — it would fail the ALTER
	// otherwise. Such a row is meaningless anyway: it hands someone a role whose
	// permissions are scoped to a workspace they aren't being granted access to.
	sqlx::query(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			(role_id, workspace_id) NOT IN (
				SELECT
					id,
					owner_id
				FROM
					role
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		DROP CONSTRAINT workspace_user_fk_role_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_fk_role_id_workspace_id
		FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, owner_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
