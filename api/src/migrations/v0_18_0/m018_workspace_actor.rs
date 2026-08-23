//! Adds `workspace_actor` — the workspace-scoped principal that role bindings
//! are granted on.
//!
//! A user acts in a workspace through their membership (`workspace_user`) and
//! gets one actor per workspace. The unique on `(user_id, workspace_id)` stops
//! a principal from holding two actors in one workspace, which would silently
//! double its bindings.
//!
//! `WORKSPACE_ACTOR_TYPE` has a single variant today, and the CHECK pins the
//! typed column that goes with it. Service accounts add a variant, a column,
//! an FK and one more arm to the CHECK — the column stays nullable so that
//! expansion is purely additive.
//!
//! Deliberately absent: the FK from `(user_id, workspace_id)` up to
//! `workspace_user` — impossible while `workspace_user`'s primary key still
//! carries `role_id`. The cutover migration collapses that table and adds the
//! FK. Nothing writes `workspace_actor` until the backfill, so the gap is
//! inert.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TYPE WORKSPACE_ACTOR_TYPE AS ENUM(
			'user'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE workspace_actor(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			actor_type WORKSPACE_ACTOR_TYPE NOT NULL,
			user_id UUID
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_actor
			ADD CONSTRAINT workspace_actor_pk PRIMARY KEY(id),
			ADD CONSTRAINT workspace_actor_uq_id_workspace_id UNIQUE(id, workspace_id),
			ADD CONSTRAINT workspace_actor_uq_user_id_workspace_id
				UNIQUE(user_id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_actor
			ADD CONSTRAINT workspace_actor_chk_type_matches_columns CHECK(
				actor_type = 'user' AND
				user_id IS NOT NULL
			),
			ADD CONSTRAINT workspace_actor_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
