use crate::prelude::*;

/// Initializes all audit log-related tables
#[instrument(skip(connection))]
pub async fn initialize_workspace_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up audit logs tables");

	query!(
		r#"
		CREATE TYPE AUDIT_LOG_TYPE AS ENUM (
			'create',
			'update',
			'delete'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE audit_log(
			id UUID NOT NULL,
			/* workspace_id is kept in case the resource is moved to another workspace */
			workspace_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			timestamp TIMESTAMPTZ NOT NULL,
			action AUDIT_LOG_TYPE NOT NULL,
			login_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes all audit log-related indices
#[instrument(skip(_connection))]
pub async fn initialize_workspace_indices(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up audit logs indices");

	Ok(())
}

/// Initializes all audit log-related constraints
#[instrument(skip(_connection))]
pub async fn initialize_workspace_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up audit logs constraints");

	Ok(())
}
