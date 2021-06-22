#[allow(clippy::module_inception)]
mod deployment;
mod docker_registry;
mod entry_point;
mod upgrade_path;

pub use self::{
	deployment::*,
	docker_registry::*,
	entry_point::*,
	upgrade_path::*,
};
use crate::Database;

pub async fn initialize_deployment_pre(
	transaction: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployment tables");
	docker_registry::initialize_docker_registry_pre(&mut *transaction).await?;
	deployment::initialize_deployment_pre(&mut *transaction).await?;
	entry_point::initialize_entry_point_pre(&mut *transaction).await?;
	upgrade_path::initialize_upgrade_path_pre(&mut *transaction).await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	transaction: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	docker_registry::initialize_docker_registry_post(&mut *transaction).await?;
	deployment::initialize_deployment_post(&mut *transaction).await?;
	entry_point::initialize_entry_point_post(&mut *transaction).await?;
	upgrade_path::initialize_upgrade_path_post(&mut *transaction).await?;

	Ok(())
}
