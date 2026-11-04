//! Index `secret_id` on deployment environment variables, so that finding the
//! deployments using a secret doesn't scan the whole table.
//!
//! - Also backs the secret FK check on delete.
//! - No data changes.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE INDEX
			deployment_environment_variable_idx_secret_id_workspace_id
		ON
			deployment_environment_variable
		(secret_id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
