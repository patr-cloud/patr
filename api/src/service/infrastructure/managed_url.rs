use api_models::{
	models::workspace::infrastructure::managed_urls::{
		ManagedUrl,
		ManagedUrlType,
	},
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use eve_rs::AsError;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use super::kubernetes;
use crate::{
	db::{self, DnsRecordType, ManagedUrlType as DbManagedUrlType},
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
				false,
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
				false,
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
				false,
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
				false,
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

	let is_configured = service::verify_managed_url_configuration(
		connection,
		&managed_url_id,
		config,
		request_id,
	)
	.await?;

	db::update_managed_url_configuration_status(
		connection,
		&managed_url_id,
		is_configured,
	)
	.await?;

	service::update_kubernetes_managed_url(
		workspace_id,
		&ManagedUrl {
			id: managed_url_id.clone(),
			sub_domain: sub_domain.to_string(),
			domain_id: domain_id.clone(),
			path: path.to_string(),
			url_type: url_type.clone(),
			is_configured,
		},
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

	service::update_kubernetes_managed_url(
		&managed_url.workspace_id,
		&ManagedUrl {
			id: managed_url.id,
			sub_domain: managed_url.sub_domain,
			domain_id: managed_url.domain_id,
			path: managed_url.path,
			url_type: match managed_url.url_type {
				DbManagedUrlType::ProxyToDeployment => {
					ManagedUrlType::ProxyDeployment {
						deployment_id: managed_url.deployment_id.status(500)?,
						port: managed_url.port.status(500)? as u16,
					}
				}
				DbManagedUrlType::ProxyToStaticSite => {
					ManagedUrlType::ProxyStaticSite {
						static_site_id: managed_url
							.static_site_id
							.status(500)?,
					}
				}
				DbManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
					url: managed_url.url.status(500)?,
				},
				DbManagedUrlType::Redirect => ManagedUrlType::Redirect {
					url: managed_url.url.status(500)?,
				},
			},
			is_configured: managed_url.is_configured,
		},
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

pub async fn verify_managed_url_configuration(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<bool, Error> {
	let managed_url = db::get_managed_url_by_id(connection, managed_url_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	let domain =
		db::get_workspace_domain_by_id(connection, &managed_url.domain_id)
			.await?
			.status(500)?;

	if !domain.is_verified {
		return Ok(false);
	}

	let configured = if domain.is_ns_internal() {
		// Domain is verified. Check if the corresponding DNS records exist
		db::get_dns_records_by_domain_id(connection, &managed_url.domain_id)
			.await?
			.into_iter()
			.any(|record| {
				let sub_domain_match = if record.name.starts_with('*') {
					managed_url.sub_domain.ends_with(&record.name[1..])
				} else {
					record.name == managed_url.sub_domain
				};
				sub_domain_match &&
					matches!(record.r#type, DnsRecordType::CNAME) &&
					record.value == "ingress.patr.cloud"
			})
	} else {
		service::create_managed_url_verification_ingress(
			&managed_url.workspace_id,
			&managed_url.id,
			&managed_url.sub_domain,
			&domain.name,
			config,
			request_id,
		)
		.await?;

		let verification_token = {
			let mut rng = thread_rng();
			(0..32)
				.map(|_| rng.sample(Alphanumeric) as char)
				.collect::<String>()
		};
		let response = reqwest::Client::new()
			.get(
				if managed_url.sub_domain == "@" {
					format!(
						"http://{}/.well-known/patr-verification",
						domain.name
					)
				} else {
					format!(
						"http://{}.{}/.well-known/patr-verification",
						managed_url.sub_domain, domain.name
					)
				},
			)
			.header(reqwest::header::AUTHORIZATION, verification_token.clone())
			.send()
			.await?
			.text()
			.await?;

		service::delete_kubernetes_managed_url_verification(
			&managed_url.workspace_id,
			&managed_url.id,
			config,
			request_id,
		)
		.await?;

		response == verification_token
	};

	Ok(configured)
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
