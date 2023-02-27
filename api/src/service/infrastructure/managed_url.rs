use api_models::{
	models::workspace::infrastructure::managed_urls::ManagedUrlType,
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use cloudflare::endpoints::zone::custom_hostname::ActivationStatus;
use eve_rs::AsError;

use crate::{
	db::{self, ManagedUrlType as DbManagedUrlType},
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
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	log::trace!("request_id: {} - Creating a new managed url with sub_domain: {} and domain_id: {} with request_id: {}",
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

	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;

	let cf_custom_hostname_id = {
		let existing_hostname = db::get_all_managed_urls_for_host(
			connection, sub_domain, domain_id,
		)
		.await?
		.into_iter()
		.next();

		match existing_hostname {
			Some(managed_url) => managed_url.cf_custom_hostname_id,
			None => {
				let (id, _status) = service::add_custom_hostname_to_cloudflare(
					&format!("{}.{}", sub_domain, domain.name),
					config,
				)
				.await?;

				id
			}
		}
	};

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
				None,
				None,
				cf_custom_hostname_id,
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
				None,
				None,
				cf_custom_hostname_id,
			)
			.await?;
		}
		ManagedUrlType::ProxyUrl { url, http_only } => {
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
				None,
				Some(*http_only),
				cf_custom_hostname_id,
			)
			.await?;
		}
		ManagedUrlType::Redirect {
			url,
			permanent_redirect,
			http_only,
		} => {
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
				Some(*permanent_redirect),
				Some(*http_only),
				cf_custom_hostname_id,
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

	service::update_cloudflare_kv_for_managed_url(
		connection, sub_domain, domain_id, config,
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
	log::trace!(
		"request_id: {} - Updating managed url with id: {} with request_id: {}",
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
				None,
				None,
			)
			.await?;
		}
		ManagedUrlType::ProxyUrl { url, http_only } => {
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
				None,
				Some(*http_only),
			)
			.await?;
		}
		ManagedUrlType::Redirect {
			url,
			permanent_redirect,
			http_only,
		} => {
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
				Some(*permanent_redirect),
				Some(*http_only),
			)
			.await?;
		}
	}

	// as of now subdomain update for managed url is not supported,
	// so we don't need to care about deleting previous host in cf kv
	service::update_cloudflare_kv_for_managed_url(
		connection,
		&managed_url.sub_domain,
		&managed_url.domain_id,
		config,
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
	log::trace!(
		"request_id: {} - Deleting managed url with id: {} with request_id: {}",
		request_id,
		managed_url_id,
		request_id
	);

	let managed_url = db::get_managed_url_by_id(connection, managed_url_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

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

	let host_deletable = db::get_all_managed_urls_for_host(
		connection,
		&managed_url.sub_domain,
		&managed_url.domain_id,
	)
	.await?
	.is_empty();

	if host_deletable {
		service::delete_custom_hostname_from_cloudflare(
			&managed_url.cf_custom_hostname_id,
			config,
		)
		.await?;
	}
	log::trace!("request_id: {} - ManagedUrl Deleted.", request_id);

	service::update_cloudflare_kv_for_managed_url(
		connection,
		&managed_url.sub_domain,
		&managed_url.domain_id,
		config,
	)
	.await?;

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

	let configured = {
		let status = service::refresh_custom_hostname_in_cloudflare(
			&managed_url.cf_custom_hostname_id,
			config,
		)
		.await?;

		if status != ActivationStatus::Active {
			log::info!(
				"request_id: {} - Custom host name is not pointed to patr fallback origin",
				request_id
			);
			false
		} else {
			true
		}
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
