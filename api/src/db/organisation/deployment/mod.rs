#[allow(clippy::module_inception)]
mod deployment;
mod docker_registry;

pub use self::{deployment::*, docker_registry::*};
use crate::Database;

pub async fn initialize_deployment_pre(
	transaction: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployment tables");
	docker_registry::initialize_docker_registry_pre(&mut *transaction).await?;
	deployment::initialize_deployment_pre(&mut *transaction).await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	transaction: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	docker_registry::initialize_docker_registry_post(&mut *transaction).await?;
	deployment::initialize_deployment_post(&mut *transaction).await?;

	Ok(())
}
