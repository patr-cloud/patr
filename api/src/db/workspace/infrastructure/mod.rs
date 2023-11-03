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

/// Initialize all infrastructure-related indexes
#[instrument(skip(connection))]
pub async fn initialize_infrastructure_indexes(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment indexes");
	deployment::initialize_deployment_indexes(connection).await?;
	managed_database::initialize_managed_database_indexes(connection).await?;
	managed_url::initialize_managed_url_indexes(connection).await?;
	static_site::initialize_static_site_indexes(connection).await?;

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
