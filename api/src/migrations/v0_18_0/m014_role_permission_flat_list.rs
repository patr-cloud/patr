//! Adds `role.is_immutable` and the `role_permission` join table.
//!
//! In the role + binding model a role is a flat list of permissions —
//! `role_permission` is that list. `is_immutable` marks the default roles
//! seeded at workspace creation, which will become uneditable and
//! undeletable. Both are additive: nothing reads or writes them until the
//! cutover, and the backfill migration fills them (keeping the fill in one
//! place so gap-window role edits can't cause drift).
//!
//! `DEFAULT FALSE` stays on the column permanently — role creation simply
//! doesn't mention it for user-created roles.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE role
		ADD COLUMN is_immutable BOOLEAN NOT NULL DEFAULT FALSE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE role_permission(
			role_id UUID NOT NULL,
			permission_id UUID NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE role_permission
		ADD CONSTRAINT role_permission_pk
		PRIMARY KEY(role_id, permission_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE role_permission
			ADD CONSTRAINT role_permission_fk_role_id
				FOREIGN KEY(role_id) REFERENCES role(id),
			ADD CONSTRAINT role_permission_fk_permission_id
				FOREIGN KEY(permission_id) REFERENCES permission(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
