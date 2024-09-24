use crate::prelude::*;

/// Initializes the runner tables
#[instrument(skip(connection))]
pub async fn initialize_runner_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up runner tables");
	query!(
		r#"
		CREATE TABLE runner(
			id UUID NOT NULL,
			name TEXT NOT NULL,
			workspace_id UUID NOT NULL,
			cloudflare_tunnel_id TEXT NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the runner indices
#[instrument(skip(connection))]
pub async fn initialize_runner_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up runner indices");
	query!(
		r#"
		ALTER TABLE runner
		ADD CONSTRAINT runner_pk
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			runner_uq_workspace_id_name
		ON
			runner(workspace_id, name)
		WHERE
			deleted IS NULL AND
			workspace_id IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the runner constraints
#[instrument(skip(connection))]
pub async fn initialize_runner_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up runner constraints");
	query!(
		r#"
		ALTER TABLE runner
			ADD CONSTRAINT runner_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT runner_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id, deleted) 
					REFERENCES resource(id, owner_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
