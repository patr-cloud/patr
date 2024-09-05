use crate::prelude::*;

/// The list of deployments that are present in a workspace
mod deployment;

/// Initializes all workspace-related tables
#[instrument(skip(connection))]
pub async fn initialize_workspace_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace tables");

	deployment::initialize_deployment_tables(connection).await?;

	Ok(())
}
