mod deployment;
mod managed_database;
mod managed_url;
mod patr_database;
mod static_site;

pub use self::{
	deployment::*,
	managed_database::*,
	managed_url::*,
	patr_database::*,
	static_site::*,
};
use crate::Database;

pub async fn initialize_infrastructure_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployment tables");
	deployment::initialize_deployment_pre(connection).await?;
	managed_database::initialize_managed_database_pre(connection).await?;
	patr_database::initialize_patr_database_pre(connection).await?;
	managed_url::initialize_managed_url_pre(connection).await?;
	static_site::initialize_static_site_pre(connection).await?;

	Ok(())
}

pub async fn initialize_infrastructure_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	deployment::initialize_deployment_post(connection).await?;
	managed_database::initialize_managed_database_post(connection).await?;
	patr_database::initialize_patr_database_post(connection).await?;
	managed_url::initialize_managed_url_post(connection).await?;
	static_site::initialize_static_site_post(connection).await?;

	Ok(())
}
