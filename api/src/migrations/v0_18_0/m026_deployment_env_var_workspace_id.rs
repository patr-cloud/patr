//! Rebuilds `deployment_environment_variable` with a `workspace_id`, so the
//! database enforces that a deployment only references secrets of its own
//! workspace.
//!
//! - `workspace_id` is backfilled from each variable's deployment.
//! - The single-column FKs on `deployment_id` and `secret_id` become ones on `(…, workspace_id)`,
//!   so a secret from another workspace can't be referenced. Rows without a `secret_id` skip the
//!   secret check.
//!
//! The table is rebuilt rather than altered so `workspace_id` sits after
//! `deployment_id`, as `initialize_deployment_tables` creates it — `ADD COLUMN`
//! would append it and leave migrated and fresh databases with different column
//! orders. The rows wait in a temporary table so the new one is created under
//! its own name, and ends up with the same constraint names as a fresh database.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TEMPORARY TABLE deployment_environment_variable_backup AS
		SELECT
			deployment_environment_variable.deployment_id,
			deployment.workspace_id,
			deployment_environment_variable.name,
			deployment_environment_variable.value,
			deployment_environment_variable.secret_id
		FROM
			deployment_environment_variable
		INNER JOIN
			deployment
		ON
			deployment.id = deployment_environment_variable.deployment_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE deployment_environment_variable;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			name VARCHAR(256) NOT NULL,
			value TEXT,
			secret_id UUID
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			deployment_environment_variable(
				deployment_id,
				workspace_id,
				name,
				value,
				secret_id
			)
		SELECT
			deployment_id,
			workspace_id,
			name,
			value,
			secret_id
		FROM
			deployment_environment_variable_backup;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE deployment_environment_variable_backup;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
		ADD CONSTRAINT deployment_environment_variable_pk
		PRIMARY KEY(deployment_id, name);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			ADD CONSTRAINT deployment_environment_variable_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id)
					REFERENCES deployment(id, workspace_id),
			ADD CONSTRAINT deployment_environment_variable_fk_secret_id_workspace_id
				FOREIGN KEY(secret_id, workspace_id)
					REFERENCES secret(id, workspace_id),
			ADD CONSTRAINT deployment_env_var_chk_value_secret_id_either_not_null CHECK(
				(
					value IS NOT NULL AND
					secret_id IS NULL
				) OR (
					value IS NULL AND
					secret_id IS NOT NULL
				)
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
