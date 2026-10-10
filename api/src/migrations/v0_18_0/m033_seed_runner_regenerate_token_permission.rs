//! Seed the `runner::regenerateToken` permission, and grant it to the default
//! runner roles that should have had it.
//!
//! The `RunnerPermission::RegenerateToken` variant exists in the enum and fresh
//! databases already seed it via `Permission::list_all()`, but no migration
//! ever inserted it — `m029` only added `serviceAccount::regenerateToken`. So
//! a database that predates the variant is missing `runner::regenerateToken`,
//! which `regenerateRunnerToken` authorizes against. This backfills that
//! permission row.
//!
//! `m020` lists it for `Runner: Editor` and `Runner: Admin`, but matched names
//! against the permission table, so where the row didn't exist yet the grant
//! was skipped. Both roles get it here, for immutable roles only, as frozen
//! names like `m020`'s.
//!
//! `ON CONFLICT DO NOTHING` throughout because a database that was initialized
//! fresh at any point already has the row and the grants, and this must be a
//! no-op there rather than a unique violation.

use crate::prelude::*;

/// The default roles that hold `runner::regenerateToken`.
const REGENERATE_TOKEN_ROLES: &[&str] = &["Runner: Editor", "Runner: Admin"];

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

	sqlx::query(
		r#"
		INSERT INTO
			role_permission(role_id, permission_id)
		SELECT
			role.id,
			permission.id
		FROM
			role
		CROSS JOIN
			permission
		WHERE
			role.name::TEXT = ANY($1) AND
			role.is_immutable = TRUE AND
			permission.name = 'runner::regenerateToken'
		ON CONFLICT
			(role_id, permission_id)
		DO NOTHING;
		"#,
	)
	.bind(REGENERATE_TOKEN_ROLES)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
