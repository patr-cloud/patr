//! Seeds the `deployment::shell` permission on existing databases. Fresh
//! installs get it automatically from `Permission::list_all()` in
//! `initialize_rbac_constraints`, but already-deployed databases were seeded
//! before the permission existed, so `get_permission_id` would panic the first
//! time a route referencing it is hit. This migration inserts the row so the
//! permission resolves everywhere.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// `ON CONFLICT DO NOTHING` keeps this idempotent and safe against a DB that
	// already happens to have the row (e.g. seeded by a newer fresh-install
	// path), matching against the unique `name`.
	sqlx::query(
		r#"
		INSERT INTO
			permission(id, name, description)
		VALUES
			(
				gen_random_uuid(),
				'deployment::shell',
				'This permission allows the user to open an interactive shell inside the running deployment (like `docker exec -it`), streaming stdin/stdout to and from the container. It grants direct runtime access to the deployment''s container, so it should be treated as a sensitive permission.'
			)
		ON CONFLICT (name) DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
