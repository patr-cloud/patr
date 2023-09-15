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
use crate::Database;

pub async fn initialize_infrastructure_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Initializing deployment tables");
	deployment::initialize_deployment_tables(connection).await?;
	managed_database::initialize_managed_database_tables(connection).await?;
	managed_url::initialize_managed_url_tables(connection).await?;
	static_site::initialize_static_site_tables(connection).await?;

	Ok(())
}

pub async fn initialize_infrastructure_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Finishing up deployment tables initialization");
	deployment::initialize_deployment_constraints(connection).await?;
	managed_database::initialize_managed_database_constraints(connection).await?;
	managed_url::initialize_managed_url_constraints(connection).await?;
	static_site::initialize_static_site_constraints(connection).await?;

	Ok(())
}
