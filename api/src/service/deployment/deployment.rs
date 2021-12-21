use std::{str, time::Duration};

use eve_rs::AsError;
use openssh::{KnownHosts, SessionBuilder};
use reqwest::Client;
use tokio::{io::AsyncWriteExt, time};
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
		deployment::{aws, digitalocean, kubernetes, CNameRecord},
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
	config: &Settings,
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

			db::begin_deferred_constraints(connection).await?;

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

			db::end_deferred_constraints(connection).await?;
		} else {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	} else if let Some(image_name) = image_name {
		db::begin_deferred_constraints(connection).await?;

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

		db::end_deferred_constraints(connection).await?;
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let image_id = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.get_full_image(connection)
		.await?;

	if check_if_image_exists_in_registry(connection, &image_id).await? {
		kubernetes::update_deployment(connection, deployment_id, config)
			.await?;
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
	let _ = deployment
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let image_id = if let Some(deployed_image) = deployment.deployed_image {
		deployed_image
	} else {
		deployment.get_full_image(connection).await?
	};

	let request_id = Uuid::new_v4();
	log::trace!("Deploying the container with id: {} and image: {} on DigitalOcean App platform with request_id: {}",
		hex::encode(&deployment_id),
		image_id,
		request_id
	);

	let _ = digitalocean::push_to_docr(
		connection,
		deployment_id,
		&image_id,
		Client::new(),
		config,
	)
	.await?;

	db::update_deployment_deployed_image(
		connection,
		deployment_id,
		Some(&image_id),
	)
	.await?;

	let result =
		kubernetes::update_deployment(connection, deployment_id, config).await;

	if let Err(error) = result {
		db::update_deployment_status(
			connection,
			deployment_id,
			&DeploymentStatus::Errored,
		)
		.await?;
		log::info!("Error with the deployment, {}", error.get_error());
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
	let _ = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - removing the deployed image info from db",
		request_id
	);
	db::update_deployment_deployed_image(connection, deployment_id, None)
		.await?;

	log::trace!(
		"request_id: {} - deleting the deployment from digitalocean kubernetes",
		request_id
	);
	kubernetes::delete_kubernetes_deployment(
		deployment_id,
		config,
		&request_id,
	)
	.await?;

	// TODO: implement logic for handling domains of the stopped deployment

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

	// TODO: implement logic for handling domains of the deleted deployment

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

	Ok(())
}

pub async fn get_deployment_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<String, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Getting deployment logs for deployment_id: {} with request_id: {}",
		hex::encode(&deployment_id),
		request_id
	);

	let _ = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	log::trace!("request_id: {} - get the deployment id from db", request_id);

	log::trace!("request_id: {} - getting logs from kubernetes", request_id);

	let logs =
		kubernetes::get_container_logs(deployment_id, request_id).await?;

	Ok(logs)
}

pub async fn set_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	environment_variables: &[(String, String)],
	config: &Settings,
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

	kubernetes::update_deployment(connection, deployment_id, config).await?;

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

async fn check_if_image_exists_in_registry(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_image_id: &str,
) -> Result<bool, Error> {
	// TODO: fill this function for checking if the user has pushed the image
	// before making the deployment if the user has pushed the image then return
	// true
	Ok(false)
}
