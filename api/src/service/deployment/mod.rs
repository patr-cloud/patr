mod aws;
mod digitalocean;

use std::{ops::DerefMut, time::Duration};

use cloudflare::{
	endpoints::{
		dns::{
			CreateDnsRecord,
			CreateDnsRecordParams,
			DnsContent,
			ListDnsRecords,
			ListDnsRecordsParams,
			UpdateDnsRecord,
			UpdateDnsRecordParams,
		},
		zone::{ListZones, ListZonesParams},
	},
	framework::{
		async_api::{ApiClient, Client as CloudflareClient},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;
use futures::StreamExt;
use openssh::{KnownHosts, Session, SessionBuilder};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use shiplift::{Docker, PullOptions, RegistryAuth, TagOptions};
use tokio::{io::AsyncWriteExt, task, time};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{
			CNameRecord,
			CloudPlatform,
			DeploymentMachineType,
			DeploymentRequestMethod,
			DeploymentRequestProtocol,
			DeploymentStatus,
			IpResponse,
			ManagedDatabaseEngine,
			ManagedDatabasePlan,
			ManagedDatabaseStatus,
		},
		rbac,
		RegistryToken,
		RegistryTokenAccess,
	},
	service,
	utils::{
		get_current_time,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
	Database,
};

/// # Description
/// This function creates a deployment under an organisation account
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `organisation_id` -  an unsigned 8 bit integer array containing the id of
///   organisation
/// * `name` - a string containing the name of deployment
/// * `registry` - a string containing the url of docker registry
/// * `repository_id` - An Option<&str> containing either a repository id of
///   type string or `None`
/// * `image_name` - An Option<&str> containing either an image name of type
///   string or `None`
/// * `image_tag` - a string containing tags of docker image
///
/// # Returns
/// This function returns Result<Uuid, Error> containing an uuid of the
/// deployment or an error
///
/// [`Transaction`]: Transaction
pub async fn create_deployment_in_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
	name: &str,
	registry: &str,
	repository_id: Option<&str>,
	image_name: Option<&str>,
	image_tag: &str,
	region: &str,
	domain_name: Option<&str>,
	horizontal_scale: u64,
	machine_type: &DeploymentMachineType,
	config: &Settings,
) -> Result<Uuid, Error> {
	// As of now, only our custom registry is allowed
	// Docker hub will also be allowed in the near future
	match registry {
		"registry.patr.cloud" => (),
		_ => {
			Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
		}
	}

	// validate deployment name
	if !validator::is_deployment_name_valid(name) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_DEPLOYMENT_NAME).to_string())?;
	}
	if let Some(domain_name) = domain_name {
		if !validator::is_deployment_entry_point_valid(domain_name) {
			return Err(Error::empty()
				.status(400)
				.body(error!(INVALID_DOMAIN_NAME).to_string()));
		}
	}

	let deployment_uuid = db::generate_new_resource_id(connection).await?;
	let deployment_id = deployment_uuid.as_bytes();

	db::create_resource(
		connection,
		deployment_id,
		&format!("Deployment: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT)
			.unwrap(),
		organisation_id,
		get_current_time_millis(),
	)
	.await?;

	if registry == "registry.patr.cloud" {
		if let Some(repository_id) = repository_id {
			let repository_id = hex::decode(repository_id)
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;

			db::create_deployment_with_internal_registry(
				connection,
				deployment_id,
				name,
				&repository_id,
				image_tag,
				region,
				domain_name,
				horizontal_scale,
				machine_type,
				organisation_id,
			)
			.await?;
		} else {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	} else if let Some(image_name) = image_name {
		db::create_deployment_with_external_registry(
			connection,
			deployment_id,
			name,
			registry,
			image_name,
			image_tag,
			region,
			domain_name,
			horizontal_scale,
			machine_type,
			organisation_id,
		)
		.await?;
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	// Deploy the app as soon as it's created, so that any existing images can
	// be deployed
	service::start_deployment(connection, deployment_id, config).await?;

	Ok(deployment_uuid)
}

pub async fn start_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let image_id = if let Some(deployed_image) = deployment.deployed_image {
		deployed_image
	} else {
		deployment.get_full_image(connection).await?
	};
	let config = config.clone();
	let region = region.to_string();
	let deployment_id = deployment.id;

	db::update_deployment_deployed_image(
		connection,
		&deployment_id,
		Some(&image_id),
	)
	.await?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			task::spawn(async move {
				let result = digitalocean::deploy_container(
					image_id,
					region,
					deployment_id.clone(),
					config,
				)
				.await;

				if let Err(error) = result {
					let _ = update_deployment_status(
						&deployment_id,
						&DeploymentStatus::Errored,
					)
					.await;
					log::info!(
						"Error with the deployment, {}",
						error.get_error()
					);
				}
			});
		}
		Ok(CloudPlatform::Aws) => {
			task::spawn(async move {
				let result = aws::deploy_container(
					image_id,
					region,
					deployment_id.clone(),
					config,
				)
				.await;

				if let Err(error) = result {
					let _ = update_deployment_status(
						&deployment_id,
						&DeploymentStatus::Errored,
					)
					.await;
					log::info!(
						"Error with the deployment, {}",
						error.get_error()
					);
				}
			});
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}

	Ok(())
}

pub async fn stop_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("Getting deployment id from db");
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!("removing the deployed image info from db");
	db::update_deployment_deployed_image(connection, deployment_id, None)
		.await?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!("deleting the deployment from digitalocean");
			digitalocean::delete_deployment(connection, deployment_id, config)
				.await?;
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("deleting the deployment from aws");
			aws::delete_deployment(connection, deployment_id, region, config)
				.await?;
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}

	let patr_domain = format!("{}.patr.cloud", hex::encode(deployment_id));
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	let mut sftp = session.sftp();

	let default_domain_ssl = session
		.command("test")
		.arg("-f")
		.arg(format!(
			"/etc/letsencrypt/live/{}/fullchain.pem",
			patr_domain
		))
		.spawn()?
		.wait()
		.await?;

	let mut writer = sftp
		.write_to(format!("/etc/nginx/sites-enabled/{}", patr_domain))
		.await?;
	writer
		.write_all(
			if default_domain_ssl.success() {
				format!(
					r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	return 301 https://{domain}$request_uri;
}}

server {{
	listen 443 ssl http2;
	listen [::]:443 ssl http2;
	server_name {domain};

	ssl_certificate /etc/letsencrypt/live/{domain}/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/{domain}/privkey.pem;
	
	root /var/www/stopped;

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
					domain = patr_domain
				)
			} else {
				format!(
					r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	root /var/www/stopped;

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
					domain = patr_domain,
				)
			}
			.as_bytes(),
		)
		.await?;
	writer.close().await?;

	if let Some(custom_domain) = deployment.domain_name {
		let custom_domain_ssl = session
			.command("test")
			.arg("-f")
			.arg(format!(
				"/etc/letsencrypt/live/{}/fullchain.pem",
				custom_domain
			))
			.spawn()?
			.wait()
			.await?;

		let mut writer = sftp
			.write_to(format!("/etc/nginx/sites-enabled/{}", custom_domain))
			.await?;
		writer
			.write_all(
				if custom_domain_ssl.success() {
					format!(
						r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	return 301 https://{domain}$request_uri;
}}

server {{
	listen 443 ssl http2;
	listen [::]:443 ssl http2;
	server_name {domain};

	ssl_certificate /etc/letsencrypt/live/{domain}/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/{domain}/privkey.pem;
	
	root /var/www/stopped;

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
						domain = custom_domain
					)
				} else {
					format!(
						r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	root /var/www/stopped;

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
						domain = custom_domain,
					)
				}
				.as_bytes(),
			)
			.await?;
		writer.close().await?;
	}

	drop(sftp);
	time::sleep(Duration::from_millis(1000)).await;

	session.close().await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	Ok(())
}

pub async fn get_deployment_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<String, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	log::trace!("get the deployment id from db");

	let (provider, _) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let logs = match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!("getting logs from digitalocean deployment");
			digitalocean::get_container_logs(connection, deployment_id, config)
				.await?
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("getting logs from aws deployment");
			aws::get_container_logs(connection, deployment_id, config).await?
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	};

	Ok(logs)
}

pub async fn create_managed_database_in_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	db_name: &str,
	engine: &ManagedDatabaseEngine,
	version: Option<&str>,
	num_nodes: Option<u64>,
	database_plan: &ManagedDatabasePlan,
	region: &str,
	organisation_id: &[u8],
	config: &Settings,
) -> Result<Uuid, Error> {
	if !validator::is_database_name_valid(db_name) {
		log::trace!("Database name is invalid. Rejecting create request");
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let (provider, region) = region
		.split_once('-')
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	log::trace!("generating new resource");
	let database_uuid = db::generate_new_resource_id(connection).await?;
	let database_id = database_uuid.as_bytes();

	let version = match engine {
		ManagedDatabaseEngine::Postgres => version.unwrap_or("12"),
		ManagedDatabaseEngine::Mysql => version.unwrap_or("8"),
	};
	let num_nodes = num_nodes.unwrap_or(1);

	db::create_resource(
		connection,
		database_id,
		&format!("{}-database-{}", provider, hex::encode(database_id)),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		organisation_id,
		get_current_time_millis(),
	)
	.await?;

	log::trace!("creating entry for newly created managed database");
	db::create_managed_database(
		connection,
		database_id,
		name,
		db_name,
		engine,
		version,
		num_nodes,
		database_plan,
		&format!("{}-{}", provider, region),
		"",
		0,
		"",
		"",
		organisation_id,
		None,
	)
	.await?;
	log::trace!("resource generation complete");

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			digitalocean::create_managed_database_cluster(
				connection,
				database_id,
				db_name,
				engine,
				version,
				num_nodes,
				database_plan,
				region,
				config,
			)
			.await?;
		}
		Ok(CloudPlatform::Aws) => {
			aws::create_managed_database_cluster(
				connection,
				database_id,
				db_name,
				engine,
				version,
				num_nodes,
				database_plan,
				region,
				config,
			)
			.await?;
		}
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	}

	Ok(database_uuid)
}

pub async fn delete_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let database = db::get_managed_database_by_id(connection, database_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (provider, region) = database
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!("deleting the database from digitalocean");
			if let Some(digitalocean_db_id) = database.digitalocean_db_id {
				digitalocean::delete_database(&digitalocean_db_id, config)
					.await?;
			}
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!("deleting the deployment from aws");
			aws::delete_database(database_id, region).await?;
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}

	db::update_managed_database_status(
		connection,
		database_id,
		&ManagedDatabaseStatus::Deleted,
	)
	.await?;
	Ok(())
}

pub async fn set_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	environment_variables: &[(String, String)],
) -> Result<(), Error> {
	db::remove_all_environment_variables_for_deployment(
		connection,
		deployment_id,
	)
	.await?;

	for (key, value) in environment_variables {
		db::add_environment_variable_for_deployment(
			connection,
			deployment_id,
			key,
			value,
		)
		.await?;
	}

	Ok(())
}

pub async fn get_dns_records_for_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Vec<CNameRecord>, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain_name = deployment
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

	Ok(vec![CNameRecord {
		cname: domain_name,
		value: "nginx.patr.cloud".to_string(), // TODO make this a config
	}])
}

pub async fn get_domain_validation_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<bool, Error> {
	log::trace!("validating the custom domain");
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain_name = deployment
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("getting the default url from db");
	let default_url = match provider.parse() {
		Ok(CloudPlatform::Aws) => {
			aws::get_app_default_url(&hex::encode(deployment_id), region)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
		}
		Ok(CloudPlatform::DigitalOcean) => {
			let client = Client::new();
			digitalocean::get_app_default_url(deployment_id, config, &client)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	};

	log::trace!("logging into the ssh server");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	log::trace!("creating random file with random content for verification");
	let (filename, file_content) =
		create_random_content_for_verification(&session).await?;

	log::trace!("checking existence of https for the custom domain");
	let https_text = reqwest::get(format!(
		"https://{}/.well-known/patr-verification/{}",
		domain_name, filename
	))
	.await
	.ok();
	if let Some(response) = https_text {
		let content = response.text().await.ok();

		if let Some(text) = content {
			session
				.command("rm")
				.arg(format!(
					"/var/www/patr-verification/.well-known/patr-verification/{}",
					filename
				))
				.spawn()?
				.wait()
				.await?;
			return Ok(text == file_content);
		}
	}

	log::trace!("https does not exist, checking for http");
	let text = reqwest::get(format!(
		"http://{}/.well-known/patr-verification/{}",
		domain_name, filename
	))
	.await?
	.text()
	.await?;

	if text == file_content {
		log::trace!("http exists creating certificate for the custom domain");

		log::trace!("checking if the certificate already exists");
		let check_file = session
			.command("test")
			.arg("-f")
			.arg(format!(
				"/etc/letsencrypt/live/{}/fullchain.pem",
				domain_name
			))
			.spawn()?
			.wait()
			.await?;

		if check_file.success() {
			log::trace!("certificate exists updating nginx config for https");
			update_nginx_config_for_domain_with_https(
				&domain_name,
				&default_url,
				config,
			)
			.await?;
			return Ok(true);
		}
		log::trace!("certificate does not exist creating a new one");
		create_https_certificates_for_domain(&domain_name, config).await?;
		log::trace!("updating nginx with https");
		update_nginx_config_for_domain_with_https(
			&domain_name,
			&default_url,
			config,
		)
		.await?;
		log::trace!("domain validated");
		return Ok(true);
	}

	session
		.command("rm")
		.arg(format!(
			"/var/www/patr-verification/.well-known/patr-verification/{}",
			filename
		))
		.spawn()?
		.wait()
		.await?;
	session.close().await?;

	Ok(false)
}

pub async fn set_domain_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
	deployment_id: &[u8],
	new_domain_name: Option<&str>,
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let old_domain = deployment.domain_name;

	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let deployment_default_url = match provider.parse() {
		Ok(CloudPlatform::Aws) => {
			aws::get_app_default_url(&hex::encode(deployment_id), region)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
		}
		Ok(CloudPlatform::DigitalOcean) => {
			let client = Client::new();
			digitalocean::get_app_default_url(deployment_id, config, &client)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	};

	db::set_domain_name_for_deployment(
		connection,
		deployment_id,
		new_domain_name,
	)
	.await?;

	match (new_domain_name, old_domain.as_deref()) {
		(None, None) => (), // Do nothing
		(Some(new_domain), None) => {
			// Setup new domain name
			let check_file = session
				.command("test")
				.arg("-f")
				.arg(format!(
					"/etc/letsencrypt/live/{}/fullchain.pem",
					new_domain
				))
				.spawn()?
				.wait()
				.await?;
			if check_file.success() {
				update_nginx_config_for_domain_with_https(
					new_domain,
					&deployment_default_url,
					config,
				)
				.await?;
			} else {
				update_nginx_config_for_domain_with_http_only(
					new_domain,
					&deployment_default_url,
					config,
				)
				.await?;
			}
		}
		(None, Some(domain_name)) => {
			session
				.command("certbot")
				.arg("delete")
				.arg("--cert-name")
				.arg(&domain_name)
				.spawn()?
				.wait()
				.await?;
			session
				.command("rm")
				.arg(format!("/etc/nginx/sites-enabled/{}", domain_name))
				.spawn()?
				.wait()
				.await?;
			session
				.command("nginx")
				.arg("-s")
				.arg("reload")
				.spawn()?
				.wait()
				.await?;
		}
		(Some(new_domain), Some(old_domain)) => {
			if new_domain != old_domain {
				session
					.command("certbot")
					.arg("delete")
					.arg("--cert-name")
					.arg(&old_domain)
					.spawn()?
					.wait()
					.await?;
				session
					.command("rm")
					.arg(format!("/etc/nginx/sites-enabled/{}", old_domain))
					.spawn()?
					.wait()
					.await?;

				let check_file = session
					.command("test")
					.arg("-f")
					.arg(format!(
						"/etc/letsencrypt/live/{}/fullchain.pem",
						new_domain
					))
					.spawn()?
					.wait()
					.await?;
				if check_file.success() {
					update_nginx_config_for_domain_with_https(
						new_domain,
						&deployment_default_url,
						config,
					)
					.await?;
				} else {
					update_nginx_config_for_domain_with_http_only(
						new_domain,
						&deployment_default_url,
						config,
					)
					.await?;
				}
			}
		}
	}
	session.close().await?;

	Ok(())
}

async fn update_nginx_with_all_domains_for_deployment(
	deployment_id_string: &str,
	default_url: &str,
	custom_domain: Option<&str>,
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("logging into the ssh server for checking certificate");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	let patr_domain = format!("{}.patr.cloud", deployment_id_string);

	log::trace!("checking if the certificates exist or not");
	let check_file = session
		.command("test")
		.arg("-f")
		.arg(format!(
			"/etc/letsencrypt/live/{}/fullchain.pem",
			patr_domain
		))
		.spawn()?
		.wait()
		.await?;

	if check_file.success() {
		log::trace!("certificate exists updating nginx config for https");
		update_nginx_config_for_domain_with_https(
			&patr_domain,
			default_url,
			config,
		)
		.await?;
	} else {
		log::trace!("certificate does not exists");
		update_nginx_config_for_domain_with_http_only(
			&patr_domain,
			default_url,
			config,
		)
		.await?;
		create_https_certificates_for_domain(&patr_domain, config).await?;
		update_nginx_config_for_domain_with_https(
			&patr_domain,
			default_url,
			config,
		)
		.await?;
	}

	if let Some(domain) = custom_domain {
		log::trace!("custom domain present, updating nginx with custom domain");
		let check_file = session
			.command("test")
			.arg("-f")
			.arg(format!("/etc/letsencrypt/live/{}/fullchain.pem", domain))
			.spawn()?
			.wait()
			.await?;
		if check_file.success() {
			update_nginx_config_for_domain_with_https(
				domain,
				default_url,
				config,
			)
			.await?;
		} else {
			update_nginx_config_for_domain_with_http_only(
				domain,
				default_url,
				config,
			)
			.await?;
		}
	}

	session.close().await?;
	Ok(())
}

async fn update_nginx_config_for_domain_with_http_only(
	domain: &str,
	default_url: &str,
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("logging into the ssh server for updating server with http");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	let mut sftp = session.sftp();

	log::trace!("successfully logged into the server");
	let mut writer = sftp
		.write_to(format!("/etc/nginx/sites-enabled/{}", domain))
		.await?;
	writer
		.write_all(
			format!(
				r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	location / {{
		proxy_pass https://{default_url};
	}}

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
				domain = domain,
				default_url = default_url,
			)
			.as_bytes(),
		)
		.await?;
	writer.close().await?;
	drop(sftp);
	time::sleep(Duration::from_millis(1000)).await;
	log::trace!("updated sites-enabled");
	let reload_result = session
		.command("nginx")
		.arg("-s")
		.arg("reload")
		.spawn()?
		.wait()
		.await?;

	if !reload_result.success() {
		return Err(Error::empty());
	}

	log::trace!("reloaded nginx");
	session.close().await?;
	log::trace!("session closed");
	Ok(())
}

async fn create_https_certificates_for_domain(
	domain: &str,
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("logging into the ssh server for adding ssl certificate");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	log::trace!("successfully logged into the server");

	log::trace!("creating certificate using certbot");
	let certificate_result = session
		.command("certbot")
		.arg("certonly")
		.arg("--agree-tos")
		.arg("-m")
		.arg("postmaster@vicara.co")
		.arg("--no-eff-email")
		.arg("-d")
		.arg(&domain)
		.arg("--webroot")
		.arg("-w")
		.arg("/var/www/letsencrypt")
		.spawn()?
		.wait()
		.await?;

	if !certificate_result.success() {
		return Err(Error::empty());
	}
	log::trace!("created certificate");
	session.close().await?;
	log::trace!("session closed");
	Ok(())
}

async fn update_nginx_config_for_domain_with_https(
	domain: &str,
	default_url: &str,
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("logging into the ssh server for updating nginx with https");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	log::trace!("successfully logged into the server");

	let mut sftp = session.sftp();

	log::trace!("updating sites-enabled for https");
	let mut writer = sftp
		.write_to(format!("/etc/nginx/sites-enabled/{}", domain))
		.await?;
	writer
		.write_all(
			format!(
				r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};

	return 301 https://{domain}$request_uri;
}}

server {{
	listen 443 ssl http2;
	listen [::]:443 ssl http2;
	server_name {domain};

	ssl_certificate /etc/letsencrypt/live/{domain}/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/{domain}/privkey.pem;
	
	location / {{
		proxy_pass https://{default_url};
	}}

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
				domain = domain,
				default_url = default_url,
			)
			.as_bytes(),
		)
		.await?;
	writer.close().await?;
	log::trace!("updated sites enabled for https");
	drop(sftp);

	let reload_result = session
		.command("nginx")
		.arg("-s")
		.arg("reload")
		.spawn()?
		.wait()
		.await?;
	if !reload_result.success() {
		return Err(Error::empty());
	}

	log::trace!("reloaded nginx");
	session.close().await?;
	Ok(())
}

async fn add_cname_record(
	sub_domain: &str,
	target: &str,
	config: &Settings,
	proxied: bool,
) -> Result<(), Error> {
	let full_domain = if sub_domain.ends_with(".patr.cloud") {
		sub_domain.to_string()
	} else {
		format!("{}.patr.cloud", sub_domain)
	};
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};
	let client = if let Ok(client) = CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};
	let zone_identifier = client
		.request(&ListZones {
			params: ListZonesParams {
				name: Some(String::from("patr.cloud")),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.next()
		.status(500)?
		.id;
	let zone_identifier = zone_identifier.as_str();
	let expected_dns_record = DnsContent::CNAME {
		content: String::from(target),
	};
	let response = client
		.request(&ListDnsRecords {
			zone_identifier,
			params: ListDnsRecordsParams {
				name: Some(full_domain.clone()),
				..Default::default()
			},
		})
		.await?;
	let dns_record = response.result.into_iter().find(|record| {
		if let DnsContent::CNAME { .. } = record.content {
			record.name == full_domain
		} else {
			false
		}
	});
	if let Some(record) = dns_record {
		if let DnsContent::CNAME { content } = record.content {
			if content != target {
				client
					.request(&UpdateDnsRecord {
						zone_identifier,
						identifier: record.id.as_str(),
						params: UpdateDnsRecordParams {
							content: expected_dns_record,
							name: &full_domain,
							proxied: Some(proxied),
							ttl: Some(1),
						},
					})
					.await?;
			}
		}
	} else {
		// Create
		client
			.request(&CreateDnsRecord {
				zone_identifier,
				params: CreateDnsRecordParams {
					content: expected_dns_record,
					name: sub_domain,
					ttl: Some(1),
					priority: None,
					proxied: Some(proxied),
				},
			})
			.await?;
	}
	Ok(())
}

async fn delete_docker_image(
	deployment_id_string: &str,
	image_name: &str,
) -> Result<(), Error> {
	let docker = Docker::new();

	docker.images().get(image_name).delete().await?;

	Ok(())
}

async fn update_deployment_status(
	deployment_id: &[u8],
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_deployment_status(
		app.database.acquire().await?.deref_mut(),
		deployment_id,
		status,
	)
	.await?;

	Ok(())
}

async fn tag_docker_image(
	image_id: &str,
	new_repo_name: &str,
) -> Result<(), Error> {
	let docker = Docker::new();
	docker
		.images()
		.get(image_id)
		.tag(
			&TagOptions::builder()
				.repo(new_repo_name)
				.tag("latest")
				.build(),
		)
		.await?;

	Ok(())
}

async fn pull_image_from_registry(
	image_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let app = service::get_app().clone();
	let god_username = db::get_user_by_user_id(
		app.database.acquire().await?.deref_mut(),
		rbac::GOD_USER_ID.get().unwrap().as_bytes(),
	)
	.await?
	.status(500)?
	.username;

	// generate token as password
	let iat = get_current_time().as_secs();
	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		iat,
		god_username.clone(),
		config,
		vec![RegistryTokenAccess {
			r#type: "repository".to_string(),
			name: image_id.to_string(),
			actions: vec!["pull".to_string()],
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der(),
	)?;

	// get token object using the above token string
	let registry_auth = RegistryAuth::builder()
		.username(god_username)
		.password(token)
		.build();

	let docker = Docker::new();
	let mut stream = docker.images().pull(
		&PullOptions::builder()
			.image(image_id)
			.auth(registry_auth)
			.build(),
	);

	while stream.next().await.is_some() {}

	Ok(())
}

async fn update_managed_database_status(
	database_id: &[u8],
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
	database_id: &[u8],
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

async fn create_random_content_for_verification(
	session: &Session,
) -> Result<(String, String), Error> {
	let filename = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(10)
		.map(char::from)
		.collect::<String>();
	let file_content = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(32)
		.map(char::from)
		.collect::<String>();
	let mut sftp = session.sftp();

	let mut writer = sftp
		.write_to(format!(
			"/var/www/patr-verification/.well-known/patr-verification/{}",
			filename
		))
		.await?;
	writer.write_all(file_content.as_bytes()).await?;
	writer.close().await?;

	drop(sftp);

	Ok((filename, file_content))
}

pub async fn create_request_log_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	timestamp: u64,
	ip_address: &str,
	method: &DeploymentRequestMethod,
	host: &str,
	protocol: &DeploymentRequestProtocol,
	path: &str,
	response_time: f64,
) -> Result<(), Error> {
	let (latitude, longitude) =
		get_location_from_ip_address(ip_address).await?;

	db::create_log_for_deployment(
		connection,
		deployment_id,
		timestamp,
		ip_address,
		latitude,
		longitude,
		method,
		host,
		protocol,
		path,
		response_time,
	)
	.await?;
	Ok(())
}

async fn get_location_from_ip_address(
	ip_address: &str,
) -> Result<(f64, f64), Error> {
	// TODO: change to https when in production
	let response = Client::new()
		.get(format!(
			"http://ip-api.com/json/{}?fields=status,message,lat,lon",
			ip_address
		))
		.send()
		.await?
		.json::<IpResponse>()
		.await?;

	if response.status != "success" {
		log::error!("{}", response.message);
		return Err(Error::empty()
			.status(400)
			.body(error!(SERVER_ERROR).to_string()));
	}
	Ok((response.lat, response.lon))
}
