use crate::prelude::*;

/// The list of deployments that are present in a workspace
mod deployment;
/// The list of managed URLs that are present in a workspace
mod managed_url;

/// Initializes all workspace-related tables
#[instrument(skip(connection))]
pub async fn initialize_workspace_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace tables");

	deployment::initialize_deployment_tables(connection).await?;
	managed_url::initialize_managed_url_tables(connection).await?;

	Ok(())
}

/// Initializes all workspace-related indices
#[instrument(skip(connection))]
pub async fn initialize_workspace_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace indices");

	deployment::initialize_deployment_indices(connection).await?;
	managed_url::initialize_managed_url_indices(connection).await?;

	Ok(())
}
