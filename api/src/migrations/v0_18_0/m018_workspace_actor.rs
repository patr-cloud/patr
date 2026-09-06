//! Adds `workspace_actor` — the workspace-scoped principal that role bindings
//! are granted on.
//!
//! The actor is the identity: `role_binding` points at it, and each concrete
//! kind of principal points back. A user acts in a workspace through their
//! membership, so `workspace_user` gains an `actor_id`. Service accounts add
//! a variant and their own table — `workspace_actor` itself never changes
//! again.
//!
//! `UNIQUE(id, actor_type)` is what lets a subtype pin the kind it belongs
//! to: `workspace_user` carries a generated `actor_type` of `'user'` and
//! references the pair, so a membership row can never attach to a service
//! account's actor. The same trick guards the include/exclude tables.
//!
//! Deliberately absent: `NOT NULL` and `UNIQUE(actor_id)`. `workspace_user`
//! still has one row per role here, so the rows of a membership share an actor
//! id and the unique would fail; and the handlers only start writing the
//! column at the cutover, which is where a fresh database tightens it too. The
//! cutover migration collapses the table and adds both.

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
			actor_type WORKSPACE_ACTOR_TYPE NOT NULL
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
			ADD CONSTRAINT workspace_actor_uq_id_actor_type UNIQUE(id, actor_type),
			ADD CONSTRAINT workspace_actor_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD COLUMN actor_id UUID DEFAULT GEN_RANDOM_UUID();
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ALTER COLUMN actor_id DROP DEFAULT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE
			workspace_user
		SET
			actor_id = membership.actor_id
		FROM
			(
				SELECT DISTINCT ON (user_id, workspace_id)
					user_id,
					workspace_id,
					actor_id
				FROM
					workspace_user
			) membership
		WHERE
			workspace_user.user_id = membership.user_id AND
			workspace_user.workspace_id = membership.workspace_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			workspace_actor(
				id,
				workspace_id,
				actor_type
			)
		SELECT DISTINCT
			actor_id,
			workspace_id,
			'user'::WORKSPACE_ACTOR_TYPE
		FROM
			workspace_user;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD COLUMN actor_type WORKSPACE_ACTOR_TYPE NOT NULL
		GENERATED ALWAYS AS ('user') STORED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
			ADD CONSTRAINT workspace_user_fk_actor_id_actor_type
				FOREIGN KEY(actor_id, actor_type)
					REFERENCES workspace_actor(id, actor_type),
			ADD CONSTRAINT workspace_user_fk_actor_id_workspace_id
				FOREIGN KEY(actor_id, workspace_id)
					REFERENCES workspace_actor(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
