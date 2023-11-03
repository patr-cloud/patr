use crate::prelude::*;

mod deployment;
mod managed_database;
mod managed_url;
mod static_site;

pub use self::{
	deployment::*,
	managed_database::*,
	managed_url::*,
	static_site::*,
};

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
