//! Adds `actor` — the workspace-scoped principal that role bindings are
//! granted on.
//!
//! A user acts in a workspace through their membership (`workspace_user`), a
//! service account (future) through its workspace-owned row; each gets one
//! `actor` per workspace. Typed columns plus the CHECK keep the discriminator
//! honest, and the per-kind uniques stop a principal from holding two actors
//! in one workspace (which would silently double its bindings).
//!
//! Deliberately absent here: the FK from `(user_id, workspace_id)` up to
//! `workspace_user` — impossible while `workspace_user`'s primary key still
//! carries `role_id`. The cutover migration collapses that table and adds
//! the FK. Nothing writes `actor` until the backfill, so the gap is inert.
//! `service_account_id` gets its FK when the `service_account` table exists.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TYPE ACTOR_TYPE AS ENUM(
			'user',
			'service_account'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE actor(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			actor_type ACTOR_TYPE NOT NULL,
			user_id UUID,
			service_account_id UUID
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor
			ADD CONSTRAINT actor_pk PRIMARY KEY(id),
			ADD CONSTRAINT actor_uq_id_workspace_id UNIQUE(id, workspace_id),
			ADD CONSTRAINT actor_uq_user_id_workspace_id UNIQUE(user_id, workspace_id),
			ADD CONSTRAINT actor_uq_service_account_id_workspace_id
				UNIQUE(service_account_id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor
			ADD CONSTRAINT actor_chk_type_matches_columns CHECK(
				(
					actor_type = 'user' AND
					user_id IS NOT NULL AND
					service_account_id IS NULL
				) OR (
					actor_type = 'service_account' AND
					service_account_id IS NOT NULL AND
					user_id IS NULL
				)
			),
			ADD CONSTRAINT actor_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
