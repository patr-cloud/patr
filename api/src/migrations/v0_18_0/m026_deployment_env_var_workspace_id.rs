//! Makes the database enforce that a deployment only references secrets of
//! its own workspace.
//!
//! - `secret` gets `UNIQUE(id, workspace_id)` so it can be the target of a composite FK.
//! - `deployment_environment_variable` gets a `workspace_id`, backfilled from its deployment.
//! - The single-column FKs on `deployment_id` and `secret_id` are replaced with ones on `(…,
//!   workspace_id)`, so a secret from another workspace can't be referenced. Rows without a
//!   `secret_id` skip the secret check.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE secret
			ADD CONSTRAINT secret_uq_id_workspace_id
				UNIQUE(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			ADD COLUMN workspace_id UUID;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE
			deployment_environment_variable
		SET
			workspace_id = deployment.workspace_id
		FROM
			deployment
		WHERE
			deployment.id = deployment_environment_variable.deployment_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			ALTER COLUMN workspace_id SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			DROP CONSTRAINT deployment_environment_variable_fk_deployment_id,
			DROP CONSTRAINT deployment_environment_variable_fk_secret_id,
			ADD CONSTRAINT deployment_environment_variable_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id)
					REFERENCES deployment(id, workspace_id),
			ADD CONSTRAINT deployment_environment_variable_fk_secret_id_workspace_id
				FOREIGN KEY(secret_id, workspace_id)
					REFERENCES secret(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
