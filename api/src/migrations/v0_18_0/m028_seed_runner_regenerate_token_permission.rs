//! Seed the `runner::regenerateToken` permission.
//!
//! The `RunnerPermission::RegenerateToken` variant exists in the enum and fresh
//! databases already seed it via `Permission::list_all()`, but no migration
//! ever inserted it — `m003` only added `serviceAccount::regenerateToken`. So
//! existing databases are missing `runner::regenerateToken`, which the
//! reconnect-runner flow authorizes against. This backfills that single
//! permission row.
//!
//! `ON CONFLICT DO NOTHING` because a database that was initialized fresh at
//! any point already has the row, and this must be a no-op there rather than
//! a unique violation.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		INSERT INTO
			permission(id, name, description)
		VALUES
			(GEN_RANDOM_UUID(), $1, $2)
		ON CONFLICT(name) DO NOTHING;
		"#,
	)
	.bind("runner::regenerateToken")
	.bind(
		"This permission allows the user to regenerate the runner token, but not \
		 view it, edit it, or delete it. This permission is useful for users or \
		 API tokens that need to only regenerate the runner token.",
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
