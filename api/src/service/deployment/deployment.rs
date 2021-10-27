use std::time::Duration;

use cloudflare::{
	endpoints::{
		dns::{
			DeleteDnsRecord,
			DnsContent,
			ListDnsRecords,
			ListDnsRecordsParams,
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
use openssh::{KnownHosts, SessionBuilder};
use reqwest::Client;
use tokio::{io::AsyncWriteExt, task, time};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{CloudPlatform, DeploymentMachineType, DeploymentStatus},
		rbac,
	},
	service::{
		self,
		deployment::{aws, digitalocean, CNameRecord},
	},
	utils::{get_current_time_millis, settings::Settings, validator, Error},
	Database,
};

/// # Description
/// This function creates a deployment under an workspace account
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `workspace_id` -  an unsigned 8 bit integer array containing the id of
///   workspace
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
pub async fn create_deployment_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
	name: &str,
	registry: &str,
	repository_id: Option<&str>,
	image_name: Option<&str>,
	image_tag: &str,
	region: &str,
	domain_name: Option<&str>,
	horizontal_scale: u64,
	machine_type: &DeploymentMachineType,
	user_id: &[u8],
) -> Result<Uuid, Error> {
	// As of now, only our custom registry is allowed
	// Docker hub will also be allowed in the near future
	let region = if region == "any" ||
		region == "anywhere" ||
		region == "ANY" ||
		region == "ANYWHERE"
	{
		"do-blr"
	} else {
		region
	};
	match registry {
		registry if registry == "registry.patr.cloud" => (),
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

	let existing_deployment =
		db::get_deployment_by_name_in_workspace(connection, name, workspace_id)
			.await?;
	if existing_deployment.is_some() {
		Error::as_result()
			.status(200)
			.body(error!(RESOURCE_EXISTS).to_string())?;
	}

	if let Some(domain_name) = domain_name {
		let is_god_user =
			user_id == rbac::GOD_USER_ID.get().unwrap().as_bytes();
		// If the entry point is not valid, OR if (the domain is special and the
		// user is not god user)
		if !validator::is_deployment_entry_point_valid(domain_name) ||
			(validator::is_domain_special(domain_name) && !is_god_user)
		{
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
		workspace_id,
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
				workspace_id,
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
			workspace_id,
		)
		.await?;
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

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
					let _ = super::update_deployment_status(
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
					let _ = super::update_deployment_status(
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"Stopping the deployment with id: {} and request_id: {}",
		hex::encode(deployment_id),
		request_id
	);
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!(
		"request_id: {} - removing the deployed image info from db",
		request_id
	);
	db::update_deployment_deployed_image(connection, deployment_id, None)
		.await?;

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!(
				"request_id: {} - deleting the deployment from digitalocean",
				request_id
			);
			digitalocean::delete_deployment(
				connection,
				deployment_id,
				config,
				request_id,
			)
			.await?;
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!(
				"request_id: {} - deleting the deployment from aws",
				request_id
			);
			aws::delete_deployment(
				connection,
				deployment_id,
				region,
				config,
				request_id,
			)
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

	log::trace!("request_id: {} - reloaded nginx", request_id);

	session.close().await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	Ok(())
}

pub async fn delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	service::stop_deployment(connection, deployment_id, config).await?;

	db::update_deployment_name(
		connection,
		deployment_id,
		&format!(
			"patr-deleted: {}-{}",
			deployment.name,
			hex::encode(deployment_id)
		),
	)
	.await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

	let patr_domain = format!("{}.patr.cloud", hex::encode(deployment_id));
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	session
		.command("rm")
		.arg(format!("/etc/nginx/sites-enabled/{}", patr_domain))
		.spawn()?
		.wait()
		.await?;

	session
		.command("certbot")
		.arg("delete")
		.arg("--cert-name")
		.arg(&patr_domain)
		.spawn()?
		.wait()
		.await?;

	if let Some(domain_name) = deployment.domain_name {
		db::begin_deferred_constraints(connection).await?;

		db::set_domain_name_for_deployment(
			connection,
			deployment_id,
			Some(format!(
				"deleted.patr.cloud.{}.{}",
				hex::encode(deployment_id),
				domain_name
			))
			.as_deref(),
		)
		.await?;

		db::end_deferred_constraints(connection).await?;

		session
			.command("rm")
			.arg(format!("/etc/nginx/sites-enabled/{}", domain_name))
			.spawn()?
			.wait()
			.await?;

		session
			.command("certbot")
			.arg("delete")
			.arg("--cert-name")
			.arg(&domain_name)
			.spawn()?
			.wait()
			.await?;
	}

	// Delete DNS Record
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

	let dns_record = client
		.request(&ListDnsRecords {
			zone_identifier,
			params: ListDnsRecordsParams {
				name: Some(patr_domain.clone()),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.find(|record| {
			if let DnsContent::CNAME { .. } = record.content {
				record.name == patr_domain
			} else {
				false
			}
		});

	if let Some(dns_record) = dns_record {
		client
			.request(&DeleteDnsRecord {
				zone_identifier,
				identifier: &dns_record.id,
			})
			.await?;
	}

	let reload_result = session
		.command("nginx")
		.arg("-s")
		.arg("reload")
		.spawn()?
		.wait_with_output()
		.await?;

	if !reload_result.status.success() {
		log::error!(
			"Unable to reload nginx config: Stdout: {:?}. Stderr: {:?}",
			String::from_utf8(reload_result.stdout),
			String::from_utf8(reload_result.stderr)
		);
		return Err(Error::empty());
	}

	Ok(())
}

pub async fn get_deployment_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<String, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Getting deployment logs for deployment_id: {} with request_id: {}",
		hex::encode(&deployment_id),
		request_id
	);

	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	log::trace!("request_id: {} - get the deployment id from db", request_id);

	let (provider, _) = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let logs = match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			log::trace!(
				"request_id: {} - getting logs from digitalocean deployment",
				request_id
			);
			digitalocean::get_container_logs(
				connection,
				deployment_id,
				config,
				request_id,
			)
			.await?
		}
		Ok(CloudPlatform::Aws) => {
			log::trace!(
				"request_id: {} - getting logs from aws deployment",
				request_id
			);
			aws::get_container_logs(
				connection,
				deployment_id,
				config,
				request_id,
			)
			.await?
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	};

	Ok(logs)
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
	config: Settings,
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
		value: config.ssh.host_name,
	}])
}

pub async fn get_domain_validation_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: &Settings,
) -> Result<bool, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Validating the deployment_id: {} with request_id: {}",
		hex::encode(&deployment_id),
		request_id
	);
	log::trace!("request_id: {} - validating the custom domain", request_id);
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

	log::trace!(
		"request_id: {} - getting the default url from db",
		request_id
	);
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

	log::trace!("request_id: {} - logging into the ssh server", request_id);
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	log::trace!("request_id: {} - creating random file with random content for verification", request_id);
	let (filename, file_content) =
		super::create_random_content_for_verification(&session).await?;

	log::trace!(
		"request_id: {} - checking existence of https for the custom domain",
		request_id
	);
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

	log::trace!(
		"request_id: {} - https does not exist, checking for http",
		request_id
	);
	let text = reqwest::get(format!(
		"http://{}/.well-known/patr-verification/{}",
		domain_name, filename
	))
	.await?
	.text()
	.await?;

	if text == file_content {
		log::trace!("request_id: {} - http exists creating certificate for the custom domain", request_id);

		log::trace!(
			"request_id: {} - checking if the certificate already exists",
			request_id
		);
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
			log::trace!("request_id: {} - certificate exists updating nginx config for https", request_id);
			update_nginx_config_for_domain_with_https(
				&domain_name,
				&default_url,
				config,
				request_id,
			)
			.await?;
			return Ok(true);
		}
		log::trace!(
			"request_id: {} - certificate does not exist creating a new one",
			request_id
		);
		super::create_https_certificates_for_domain(
			&domain_name,
			config,
			request_id,
		)
		.await?;
		log::trace!("request_id: {} - updating nginx with https", request_id);
		update_nginx_config_for_domain_with_https(
			&domain_name,
			&default_url,
			config,
			request_id,
		)
		.await?;
		log::trace!("request_id: {} - domain validated", request_id);
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"Deploying the static site with id: {} and request_id: {}",
		hex::encode(deployment_id),
		request_id
	);
	log::trace!(
		"request_id: {} - getting deployment info from database",
		request_id
	);
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	let old_domain = deployment.domain_name;

	log::trace!("request_id: {} - logging into the ssh server for adding a new domain name for deployment", request_id);
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

	log::trace!(
		"request_id: {} - getting default url from providers",
		request_id
	);
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

	log::trace!(
		"request_id: {} - updating database with new domain",
		request_id
	);
	db::begin_deferred_constraints(connection).await?;

	db::set_domain_name_for_deployment(
		connection,
		deployment_id,
		new_domain_name,
	)
	.await?;

	db::end_deferred_constraints(connection).await?;

	match (new_domain_name, old_domain.as_deref()) {
		(None, None) => {
			log::trace!("request_id: {} - both domains are null", request_id);
		} // Do nothing
		(Some(new_domain), None) => {
			log::trace!(
				"request_id: {} - old domain null, adding new domain",
				request_id
			);
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
				log::trace!(
					"certificate present, updating nginx config with https"
				);
				update_nginx_config_for_domain_with_https(
					new_domain,
					&deployment_default_url,
					config,
					request_id,
				)
				.await?;
			} else {
				log::trace!("request_id: {} - certificate not present updating nginx with http", request_id);
				update_nginx_config_for_domain_with_http_only(
					new_domain,
					&deployment_default_url,
					config,
					request_id,
				)
				.await?;
			}
		}
		(None, Some(domain_name)) => {
			log::trace!(
				"request_id: {} - new domain null, deleting old domain",
				request_id
			);
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
			log::trace!(
				"request_id: {} - replacing old domain with new domain",
				request_id
			);
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
					log::trace!("request_id: {} - certificate creation successfull updating nginx with https", request_id);
					update_nginx_config_for_domain_with_https(
						new_domain,
						&deployment_default_url,
						config,
						request_id,
					)
					.await?;
				} else {
					log::trace!(
						"certificate creation failed updating nginx with http"
					);
					update_nginx_config_for_domain_with_http_only(
						new_domain,
						&deployment_default_url,
						config,
						request_id,
					)
					.await?;
				}
			}
		}
	}
	session.close().await?;
	log::trace!("request_id: {} - session closed)", request_id);
	log::trace!("request_id: {} - domains updated successfully", request_id);

	Ok(())
}

pub(super) async fn update_nginx_with_all_domains_for_deployment(
	deployment_id_string: &str,
	default_url: &str,
	custom_domain: Option<&str>,
	config: &Settings,
	request_id: Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - logging into the ssh server for checking certificate",
		request_id
	);
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	let patr_domain = format!("{}.patr.cloud", deployment_id_string);

	log::trace!(
		"request_id: {} - checking if the certificates exist or not",
		request_id
	);
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
		log::trace!("request_id: {} - certificate exists updating nginx config for https", request_id);
		update_nginx_config_for_domain_with_https(
			&patr_domain,
			default_url,
			config,
			request_id,
		)
		.await?;
	} else {
		log::trace!("request_id: {} - certificate does not exists", request_id);
		update_nginx_config_for_domain_with_http_only(
			&patr_domain,
			default_url,
			config,
			request_id,
		)
		.await?;
		super::create_https_certificates_for_domain(
			&patr_domain,
			config,
			request_id,
		)
		.await?;
		update_nginx_config_for_domain_with_https(
			&patr_domain,
			default_url,
			config,
			request_id,
		)
		.await?;
	}

	if let Some(domain) = custom_domain {
		log::trace!("request_id: {} - custom domain present, updating nginx with custom domain", request_id);
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
				request_id,
			)
			.await?;
		} else {
			update_nginx_config_for_domain_with_http_only(
				domain,
				default_url,
				config,
				request_id,
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
	request_id: Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - logging into the ssh server for updating server with http", request_id);
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	let mut sftp = session.sftp();

	log::trace!(
		"request_id: {} - successfully logged into the server",
		request_id
	);
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
	log::trace!("request_id: {} - updated sites-enabled", request_id);
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

	log::trace!("request_id: {} - reloaded nginx", request_id);
	session.close().await?;
	log::trace!("request_id: {} - session closed", request_id);
	Ok(())
}

async fn update_nginx_config_for_domain_with_https(
	domain: &str,
	default_url: &str,
	config: &Settings,
	request_id: Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - logging into the ssh server for updating nginx with https", request_id);
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	log::trace!(
		"request_id: {} - successfully logged into the server",
		request_id
	);

	let mut sftp = session.sftp();

	log::trace!(
		"request_id: {} - updating sites-enabled for https",
		request_id
	);
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
	log::trace!(
		"request_id: {} - updated sites-enabled for https",
		request_id
	);
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

	log::trace!("request_id: {} - reloaded nginx", request_id);
	session.close().await?;
	Ok(())
}
