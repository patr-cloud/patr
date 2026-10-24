//! Seed the `runner::regenerateToken` permission, grant it to the default
//! runner roles that should have had it, and delete `runner::execute`.
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
//!
//! `runner::execute` is gone: only a runner's own token may act as that
//! runner, and the routes it reaches check who the caller is rather than a
//! permission. Its `role_permission` and `user_api_token_permission_binding`
//! rows go first, then the permission itself, and `Runner: Editor`'s
//! description stops promising it. Creating a runner or regenerating its
//! token mints a token that reads the whole workspace, so both permissions'
//! descriptions now say so.

use crate::prelude::*;

/// The default roles that hold `runner::regenerateToken`.
const REGENERATE_TOKEN_ROLES: &[&str] = &["Runner: Editor", "Runner: Admin"];

/// The permission descriptions rewritten here, as they were worded when this
/// migration was written.
const PERMISSION_DESCRIPTIONS: &[(&str, &str)] = &[
	(
		"runner::create",
		"This permission allows the user to create a new runner in a workspace, but not view, \
		 edit, or delete it. Creating a runner mints its token, which can read every secret, \
		 container image and deployment in the workspace, so this permission effectively grants \
		 all of that as well.",
	),
	(
		"runner::regenerateToken",
		"This permission allows the user to regenerate the runner token, but not view it, edit \
		 it, or delete it. The new token can read every secret, container image and deployment \
		 in the workspace, so this permission effectively grants all of that as well.",
	),
];

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

	for (name, description) in PERMISSION_DESCRIPTIONS {
		sqlx::query(
			r#"
			UPDATE
				permission
			SET
				description = $2
			WHERE
				name = $1;
			"#,
		)
		.bind(name)
		.bind(description)
		.execute(&mut *connection)
		.await?;
	}

	sqlx::query(
		r#"
		DELETE FROM
			role_permission
		USING
			permission
		WHERE
			role_permission.permission_id = permission.id AND
			permission.name = 'runner::execute';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			user_api_token_permission_binding
		USING
			permission
		WHERE
			user_api_token_permission_binding.permission_id = permission.id AND
			permission.name = 'runner::execute';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			permission
		WHERE
			name = 'runner::execute';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE
			role
		SET
			description = 'Default role: create, edit, and regenerate tokens on runners.'
		WHERE
			name = 'Runner: Editor' AND
			is_immutable = TRUE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
