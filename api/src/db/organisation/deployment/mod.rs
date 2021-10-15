#[allow(clippy::module_inception)]
mod deployment;
mod docker_registry;
mod managed_database;
mod static_site;

pub use self::{
	deployment::*,
	docker_registry::*,
	managed_database::*,
	static_site::*,
};
use crate::Database;

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployment tables");
	docker_registry::initialize_docker_registry_pre(connection).await?;
	deployment::initialize_deployment_pre(connection).await?;
	managed_database::initialize_managed_database_pre(connection).await?;
	static_site::initialize_static_sites_pre(connection).await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	docker_registry::initialize_docker_registry_post(connection).await?;
	deployment::initialize_deployment_post(connection).await?;
	managed_database::initialize_managed_database_post(connection).await?;
	static_site::initialize_static_sites_post(connection).await?;

	Ok(())
}
