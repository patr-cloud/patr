use api_models::{
	models::workspace::infrastructure::managed_urls::{
		ManagedUrl,
		ManagedUrlType,
	},
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use eve_rs::AsError;
use redis::AsyncCommands;

use super::kubernetes;
use crate::{
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	models::rbac,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn create_new_managed_url_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	sub_domain: &str,
	domain_id: &Uuid,
	path: &str,
	url_type: &ManagedUrlType,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	log::trace!("request_id: {} - Creating a new managed url with sub_domain: {} and domain_id: {} on Kubernetes with request_id: {}",
		request_id,
		sub_domain,
		domain_id,
		request_id
	);

	let managed_url_id = db::generate_new_resource_id(connection).await?;

	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;

	log::trace!("request_id: {} - Checking resource limit", request_id);
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	log::trace!("request_id: {} - Checking managed_url limit", request_id);
	if managed_url_limit_crossed(connection, workspace_id, request_id).await? {
		return Error::as_result()
			.status(400)
			.body(error!(MANAGED_URL_LIMIT_EXCEEDED).to_string())?;
	}

	let creation_time = Utc::now();
	log::trace!("request_id: {} - Creating resource.", request_id);
	db::create_resource(
		connection,
		&managed_url_id,
		&format!("Managed URL: {}", managed_url_id),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_URL)
			.unwrap(),
		workspace_id,
		creation_time.timestamp_millis() as u64,
	)
	.await?;

	log::trace!("request_id: {} - Creating managed url.", request_id);
	match url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => {
			log::trace!(
				"request_id: {} - Creating managed url for proxyDeployment.",
				request_id
			);
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::ProxyToDeployment,
				Some(deployment_id),
				Some(*port),
				None,
				None,
				workspace_id,
			)
			.await?;
		}
		ManagedUrlType::ProxyStaticSite { static_site_id } => {
			log::trace!(
				"request_id: {} - Creating managed url for proxyStaticSite.",
				request_id
			);
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::ProxyToStaticSite,
				None,
				None,
				Some(static_site_id),
				None,
				workspace_id,
			)
			.await?;
		}
		ManagedUrlType::ProxyUrl { url } => {
			log::trace!(
				"request_id: {} - Creating managed url for proxyUrl.",
				request_id
			);
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::ProxyUrl,
				None,
				None,
				None,
				Some(url),
				workspace_id,
			)
			.await?;
		}
		ManagedUrlType::Redirect { url } => {
			log::trace!(
				"request_id: {} - Creating managed url for redirect.",
				request_id
			);
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::Redirect,
				None,
				None,
				None,
				Some(url),
				workspace_id,
			)
			.await?;
		}
	}

	let num_managed_urls =
		db::get_all_managed_urls_in_workspace(connection, workspace_id)
			.await?
			.len();
	db::update_managed_url_usage_history(
		connection,
		workspace_id,
		&(num_managed_urls as i32),
		&DateTime::from(creation_time),
	)
	.await?;

	if domain.is_ns_external() && domain.is_verified {
		log::trace!(
			"request_id: {} - Creating certificates for managed url.",
			request_id
		);

		// Randomly generate a secret content for the TLS certificate
		let verification_secret = Uuid::new_v4();

		let app = service::get_app();
		let mut redis = app.redis.clone();
		redis
			.set(
				format!("verfication-{}", managed_url_id),
				verification_secret.to_string(),
			)
			.await?;
		let host = if sub_domain == "@" {
			domain.name.clone()
		} else {
			format!("{}.{}", sub_domain, domain.name)
		};
		kubernetes::verify_managed_url(
			workspace_id,
			&domain,
			&ManagedUrl {
				id: managed_url_id.clone(),
				sub_domain: sub_domain.to_string(),
				domain_id: domain.id.clone(),
				path: path.to_string(),
				url_type: url_type.clone(),
			},
			&host,
			verification_secret.as_str(),
			config,
			request_id,
		)
		.await?;
	}
	log::trace!(
		"request_id: {} - Queuing update kubernetes managed url",
		request_id
	);
	service::queue_create_managed_url(
		workspace_id,
		&managed_url_id,
		config,
		request_id,
	)
	.await?;
	log::trace!("request_id: {} - ManagedUrl Created.", request_id);
	Ok(managed_url_id)
}

pub async fn update_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
	path: &str,
	url_type: &ManagedUrlType,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Updating managed url with id: {} on Kubernetes with request_id: {}",
		request_id,
		managed_url_id,
		request_id
	);

	let managed_url = db::get_managed_url_by_id(connection, managed_url_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	match url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => {
			log::trace!(
				"request_id: {} - Updating managed url for proxyDeployment.",
				request_id
			);
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::ProxyToDeployment,
				Some(deployment_id),
				Some(*port),
				None,
				None,
			)
			.await?;
		}
		ManagedUrlType::ProxyStaticSite { static_site_id } => {
			log::trace!(
				"request_id: {} - Updating managed url for proxyStaticSite.",
				request_id
			);
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::ProxyToStaticSite,
				None,
				None,
				Some(static_site_id),
				None,
			)
			.await?;
		}
		ManagedUrlType::ProxyUrl { url } => {
			log::trace!(
				"request_id: {} - Updating managed url for proxyUrl.",
				request_id
			);
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::ProxyUrl,
				None,
				None,
				None,
				Some(url),
			)
			.await?;
		}
		ManagedUrlType::Redirect { url } => {
			log::trace!(
				"request_id: {} - Updating managed url for redirect.",
				request_id
			);
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::Redirect,
				None,
				None,
				None,
				Some(url),
			)
			.await?;
		}
	}

	log::trace!(
		"request_id: {} - Queuing update kubernetes managed url",
		request_id
	);
	service::queue_create_managed_url(
		&managed_url.workspace_id,
		managed_url_id,
		config,
		request_id,
	)
	.await?;

	log::trace!("request_id: {} - ManagedUrl Updated.", request_id);
	Ok(())
}

pub async fn delete_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	managed_url_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Deleting managed url with id: {} on Kubernetes with request_id: {}",
		request_id,
		managed_url_id,
		request_id
	);

	let managed_url = db::get_managed_url_by_id(connection, managed_url_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain =
		db::get_workspace_domain_by_id(connection, &managed_url.domain_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

	db::update_managed_url_sub_domain(
		connection,
		managed_url_id,
		&format!(
			"patr-deleted: {}@{}",
			managed_url.id, managed_url.sub_domain
		),
	)
	.await?;

	let num_managed_urls =
		db::get_all_managed_urls_in_workspace(connection, workspace_id)
			.await?
			.len();
	db::update_managed_url_usage_history(
		connection,
		workspace_id,
		&(num_managed_urls as i32),
		&DateTime::from(Utc::now()),
	)
	.await?;

	log::trace!(
		"request_id: {} - Deleting managed url on Kubernetes.",
		request_id
	);
	kubernetes::delete_kubernetes_managed_url(
		workspace_id,
		managed_url_id,
		config,
		&Uuid::new_v4(),
	)
	.await?;

	if domain.is_ns_external() {
		log::trace!(
			"request_id: {} - Deleting certificates for external managed url",
			request_id
		);
		let secret_name = format!("tls-{}", managed_url.id);
		let certificate_name = format!("certificate-{}", managed_url.id);

		log::trace!(
			"request_id: {} - Deleting certificate for external managed url",
			request_id
		);
		service::delete_certificates_for_domain(
			workspace_id,
			&certificate_name,
			&secret_name,
			config,
			request_id,
		)
		.await?;
	}
	log::trace!("request_id: {} - ManagedUrl Deleted.", request_id);

	Ok(())
}

async fn managed_url_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!(
		"request_id: {} - Checking if free limits are crossed",
		request_id
	);

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let current_managed_urls =
		db::get_all_managed_urls_in_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!(
		"request_id: {} - Checking if managed url limits are crossed",
		request_id
	);
	if current_managed_urls + 1 > workspace.managed_url_limit as usize {
		return Ok(true);
	}

	Ok(false)
}
