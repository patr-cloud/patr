mod deployment;
mod digitalocean;
mod kubernetes;
mod managed_database;
mod managed_url;
mod secret;
mod static_site;

use std::ops::DerefMut;

use api_models::utils::Uuid;

pub use self::{
	deployment::*,
	kubernetes::*,
	managed_database::*,
	managed_url::*,
	secret::*,
	static_site::*,
};
use crate::{
	db::{self, ManagedDatabaseStatus},
	service,
	Database,
};

async fn update_managed_database_status(
	database_id: &Uuid,
	status: &ManagedDatabaseStatus,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		database_id,
		status,
	)
	.await?;

	Ok(())
}

async fn update_managed_database_credentials_for_database(
	database_id: &Uuid,
	host: &str,
	port: i32,
	username: &str,
	password: &str,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_credentials_for_database(
		app.database.acquire().await?.deref_mut(),
		database_id,
		host,
		port,
		username,
		password,
	)
	.await?;

	Ok(())
}

pub async fn resource_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<bool, sqlx::Error> {
	log::trace!(
		"request_id: {} - retreiving current deployments",
		request_id
	);
	let current_resource =
		db::get_deployments_for_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!(
		"request_id: {} - retreiving current static sites",
		request_id
	);
	let current_resource = current_resource +
		db::get_static_sites_for_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!("request_id: {} - retreiving current databases", request_id);
	let current_resource = current_resource +
		db::get_all_database_clusters_for_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!(
		"request_id: {} - retreiving current managed urls",
		request_id
	);
	let current_resource = current_resource +
		db::get_all_managed_urls_in_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!("request_id: {} - retreiving current secrets", request_id);
	let current_resource = current_resource +
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!(
		"request_id: {} - retreiving current resource limit for workspace",
		request_id
	);
	let resource_limit =
		db::get_resource_limit_for_workspace(connection, workspace_id).await?;

	if current_resource + 1 > resource_limit as usize {
		return Ok(true);
	}

	Ok(false)
}
