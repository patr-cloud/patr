use crate::prelude::*;

/// The list of deployments that are present in a workspace
mod deployment;
/// The list of databases that are created in a workspace
mod managed_database;
/// The list of Managed URLs that are created in a workspace
mod managed_url;
/// The list of static sites that are created in a workspace
mod static_site;

/// Initialize all infrastructure-related tables
#[instrument(skip(connection))]
pub async fn initialize_infrastructure_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace tables");
	deployment::initialize_deployment_tables(connection).await?;
	managed_database::initialize_managed_database_tables(connection).await?;
	managed_url::initialize_managed_url_tables(connection).await?;
	static_site::initialize_static_site_tables(connection).await?;

	Ok(())
}

/// Initialize all infrastructure-related indices
#[instrument(skip(connection))]
pub async fn initialize_infrastructure_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment indices");
	deployment::initialize_deployment_indices(connection).await?;
	managed_database::initialize_managed_database_indices(connection).await?;
	managed_url::initialize_managed_url_indices(connection).await?;
	static_site::initialize_static_site_indices(connection).await?;

	Ok(())
}

/// Initialize all infrastructure-related constraints
#[instrument(skip(connection))]
pub async fn initialize_infrastructure_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment constraints");
	deployment::initialize_deployment_constraints(connection).await?;
	managed_database::initialize_managed_database_constraints(connection).await?;
	managed_url::initialize_managed_url_constraints(connection).await?;
	static_site::initialize_static_site_constraints(connection).await?;

	Ok(())
}
