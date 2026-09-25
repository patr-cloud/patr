//! Recreates `secret` in the shape `initialize_secret_*` builds on a fresh
//! database.
//!
//! - Adds `last_updated`, before `deleted` — adding the column in place would append it after
//!   `deleted` and leave migrated and fresh databases with different column orders.
//! - Adds `secret_uq_id_workspace_id`, so the table can be the target of a composite FK.
//!
//! Secrets were never shipped, so the table holds no data and is dropped
//! rather than rebuilt. `deployment_environment_variable_fk_secret_id` is the
//! only FK into it; it's dropped first and restored as it was.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			DROP CONSTRAINT deployment_environment_variable_fk_secret_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE secret;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE secret(
			id UUID NOT NULL,
			name CITEXT NOT NULL,
			workspace_id UUID NOT NULL,
			last_updated TIMESTAMPTZ NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE secret
			ADD CONSTRAINT secret_pk PRIMARY KEY(id),
			ADD CONSTRAINT secret_uq_id_workspace_id
				UNIQUE(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			secret_uq_workspace_id_name
		ON
			secret(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE secret
			ADD CONSTRAINT secret_chk_name_is_trimmed CHECK(name = TRIM(name)),
			ADD CONSTRAINT secret_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, workspace_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			ADD CONSTRAINT deployment_environment_variable_fk_secret_id
				FOREIGN KEY(secret_id) REFERENCES secret(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
