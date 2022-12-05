use std::time::Instant;

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
	utils::{constants::free_limits, settings::Settings, Error},
	Database,
};

pub async fn create_new_managed_url_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	sub_domain: &str,
	domain_id: &Uuid,
	path: &str,
	url_type: &ManagedUrlType,
	permanent_redirect: bool,
	ssl_redirect: bool,
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

	check_managed_url_creation_limit(connection, workspace_id, request_id)
		.await?;

	let creation_time = Utc::now();
	log::trace!("request_id: {} - Creating resource.", request_id);
	db::create_resource(
		connection,
		&managed_url_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_URL)
			.unwrap(),
		workspace_id,
		&creation_time,
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
				permanent_redirect,
				ssl_redirect,
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
				permanent_redirect,
				ssl_redirect,
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
				false,
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
				permanent_redirect,
				ssl_redirect,
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
			permanent_redirect,
			ssl_redirect,
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
	permanent_redirect: bool,
	ssl_redirect: bool,
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
				permanent_redirect,
				ssl_redirect,
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
				permanent_redirect,
				ssl_redirect,
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
				false,
				false,
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
				permanent_redirect,
				ssl_redirect,
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
			path: path.to_string(),
			url_type: url_type.clone(),
			is_configured: managed_url.is_configured,
			permanent_redirect,
			ssl_redirect,
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

	db::delete_managed_url(connection, managed_url_id, &Utc::now()).await?;

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
		let verification_token = {
			let mut rng = thread_rng();
			(0..32)
				.map(|_| rng.sample(Alphanumeric) as char)
				.collect::<String>()
		};
		service::create_managed_url_verification_ingress(
			&managed_url.workspace_id,
			&managed_url.id,
			&managed_url.sub_domain,
			&domain.name,
			&verification_token,
			config,
			request_id,
		)
		.await?;
		let time = Instant::now();

		let mut response = String::with_capacity(32);
		let mut index = 0;

		while response != verification_token {
			log::trace!("Verification token not found. Retrying...");
			tokio::time::sleep(std::time::Duration::from_millis(500)).await;
			index += 1;

			response.clear();
			response.push_str(
				&reqwest::Client::builder()
					.build()?
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
					.body(verification_token.as_bytes().to_vec())
					.send()
					.await?
					.text()
					.await?,
			);

			if index > 10 {
				break;
			}
		}
		log::trace!(
			"Verification token found after {} ms",
			time.elapsed().as_millis()
		);

		log::trace!("Deleting managed urls verification ingress");

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

async fn check_managed_url_creation_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Checking whether new managed url creation is limited");

	let current_managed_url_count =
		db::get_all_managed_urls_in_workspace(connection, workspace_id)
			.await?
			.len();

	// check whether free limit is exceeded
	if current_managed_url_count >= free_limits::MANAGED_URL_COUNT &&
		db::get_default_payment_method_for_workspace(
			connection,
			workspace_id,
		)
		.await?
		.is_none()
	{
		log::info!(
			"request_id: {request_id} - Free managed url limit reached and card is not added"
		);
		return Error::as_result()
			.status(400)
			.body(error!(CARDLESS_FREE_LIMIT_EXCEEDED).to_string())?;
	}

	// check whether max managed url limit is exceeded
	let max_managed_url_limit =
		db::get_workspace_info(connection, workspace_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
			.managed_url_limit;
	if current_managed_url_count >= max_managed_url_limit as usize {
		log::info!(
			"request_id: {request_id} - Max managed url limit for workspace reached"
		);
		return Error::as_result()
			.status(400)
			.body(error!(MANAGED_URL_LIMIT_EXCEEDED).to_string())?;
	}

	// check whether total resource limit is exceeded
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		log::info!("request_id: {request_id} - Total resource limit exceeded");
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	Ok(())
}
