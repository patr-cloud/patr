mod deployment;
mod digitalocean;
mod kubernetes;
mod managed_database;
mod managed_url;
mod secret;
mod static_site;

use std::ops::DerefMut;

use api_models::utils::Uuid;
use chrono::Utc;
use eve_rs::AsError;

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
	utils::{settings::Settings, Error},
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

pub async fn delete_all_resources_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} deleting all resources in workspace: {}",
		request_id,
		workspace_id
	);
	// Get managed url and delete all the managed url for a workspace
	log::trace!("deleting all mananged urls for workspace: {}", workspace_id);
	let managed_url =
		db::get_all_managed_urls_in_workspace(connection, workspace_id).await?;

	for url in managed_url {
		service::delete_managed_url(
			connection,
			workspace_id,
			&url.id,
			config,
			request_id,
		)
		.await?;
	}

	// Get domain and delete all the domain for a workspace
	log::trace!("deleting all domains for workspace: {}", workspace_id);
	let domains =
		db::get_domains_for_workspace(connection, workspace_id).await?;

	for domain in domains {
		service::delete_domain_in_workspace(
			connection,
			workspace_id,
			&domain.id,
			config,
			request_id,
		)
		.await?;
	}

	log::trace!("deleting all databases for workspace: {}", workspace_id);
	let managed_database =
		db::get_all_database_clusters_for_workspace(connection, workspace_id)
			.await?;

	// Get managed databases and delete all the managed databases for a
	// workspace
	for database in managed_database {
		service::delete_managed_database(
			connection,
			&database.id,
			config,
			request_id,
		)
		.await?;
	}

	// Get deployments and delete all the deployment for a workspace
	log::trace!("deleting all deployments for workspace: {}", workspace_id);
	let deployments =
		db::get_deployments_for_workspace(connection, workspace_id).await?;

	for deployment in deployments {
		let region = db::get_region_by_id(connection, &deployment.region)
			.await?
			.status(500)?;

		let delete_k8s_resource = region.is_patr_region();
		service::delete_deployment(
			connection,
			&deployment.workspace_id,
			&deployment.id,
			&deployment.region,
			None,
			None,
			"0.0.0.0",
			true,
			delete_k8s_resource,
			config,
			request_id,
		)
		.await?;
	}

	// Get static sites and delete all the static site for a workspace
	log::trace!("deleting all static site for workspace: {}", workspace_id);
	let static_site =
		db::get_static_sites_for_workspace(connection, workspace_id).await?;

	for site in static_site {
		service::delete_static_site(
			connection,
			workspace_id,
			&site.id,
			config,
			request_id,
		)
		.await?;
	}

	// Get docker repositories and delete all the docker repositories for a
	// workspace
	log::trace!(
		"deleting all docker repository for workspace: {}",
		workspace_id
	);
	let docker_repositories =
		db::get_docker_repositories_for_workspace(connection, workspace_id)
			.await?;

	for (repo, ..) in docker_repositories {
		service::delete_docker_repository(
			connection, &repo.id, config, request_id,
		)
		.await?;
	}

	// Get connected_git_provider and delete all the connected git providers for
	// a workspace
	log::trace!("deleting all git provider for workspace: {}", workspace_id);
	let connected_git_providers =
		db::list_connected_git_providers_for_workspace(
			connection,
			workspace_id,
		)
		.await?;

	for git_provider in connected_git_providers {
		db::remove_git_provider_credentials(connection, &git_provider.id)
			.await?;
	}

	log::trace!("deleting all secrets for workspace: {}", workspace_id);
	let secrets =
		db::get_all_secrets_in_workspace(connection, workspace_id).await?;

	for secret in secrets {
		service::delete_secret_in_workspace(
			connection,
			workspace_id,
			&secret.id,
			config,
			request_id,
		)
		.await?;
	}

	// all the deployments are already deleted,
	// so only need to delete the region alone from db
	log::trace!("deleting all regions for workspace: {}", workspace_id);
	let regions =
		db::get_all_deployment_regions_for_workspace(connection, workspace_id)
			.await?;
	for region in regions {
		db::delete_region(connection, &region.id, &Utc::now()).await?;
	}

	log::trace!(
		"request_id: {} successfully  deleted all resources in workspace: {}",
		request_id,
		workspace_id
	);

	Ok(())
}
