#[allow(clippy::module_inception)]
mod deployment;
mod docker_registry;

pub use self::{deployment::*, docker_registry::*};
use crate::Database;

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployment tables");
	docker_registry::initialize_docker_registry_pre(connection).await?;
	deployment::initialize_deployment_pre(connection).await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	docker_registry::initialize_docker_registry_post(connection).await?;
	deployment::initialize_deployment_post(connection).await?;

	Ok(())
}
