//! Simplify the `workspace_uq_name` unique index from `LOWER(name)` to a plain
//! `(name)`.
//!
//! `workspace.name` is CITEXT, which already folds case in equality and
//! uniqueness, so the `LOWER(name)` functional index is redundant — `(name)`
//! enforces the exact same `Acme == acme` collision. The functional index also
//! can't serve a `name = $1` equality lookup (the planner won't match a CITEXT
//! equality to a `LOWER()` expression index), whereas a plain index on the
//! CITEXT column can — so `is_name_available` and the create-time availability
//! check become index-backed. The partial `WHERE deleted IS NULL` is preserved.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DROP INDEX IF EXISTS workspace_uq_name;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			workspace_uq_name
		ON
			workspace(name)
		WHERE
			deleted IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
