use std::{
	io::{Cursor, Read},
	ops::DerefMut,
	time::Duration,
};

use async_zip::read::seek::ZipFileReader;
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
use s3::creds;
use tokio::{io::AsyncWriteExt, task, time};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{CNameRecord, DeploymentStatus},
		deployment::cloud_providers::digitalocean::Storage,
		rbac,
	},
	service::{
		self,
		deployment::{self, kubernetes},
	},
	utils::{get_current_time_millis, settings::Settings, validator, Error},
	Database,
};

pub async fn create_static_site_deployment_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
	name: &str,
	domain_name: Option<&str>,
	user_id: &[u8],
) -> Result<Uuid, Error> {
	// validate static site name
	if !validator::is_deployment_name_valid(name) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_DEPLOYMENT_NAME).to_string())?;
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

	let existing_static_site = db::get_static_site_by_name_in_workspace(
		connection,
		name,
		workspace_id,
	)
	.await?;
	if existing_static_site.is_some() {
		Error::as_result()
			.status(200)
			.body(error!(RESOURCE_EXISTS).to_string())?;
	}

	let static_uuid = db::generate_new_resource_id(connection).await?;
	let static_site_id = static_uuid.as_bytes();
	log::trace!("creating resource");
	db::create_resource(
		connection,
		static_site_id,
		&format!("Static_site: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::STATIC_SITE)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("Adding entry to database");
	db::create_static_site(
		connection,
		static_site_id,
		name,
		domain_name,
		workspace_id,
	)
	.await?;

	Ok(static_uuid)
}

pub async fn start_static_site_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
	file: Option<&str>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Deploying the static site with id: {} and request_id: {}",
		hex::encode(&static_site_id),
		request_id
	);

	if let Some(file) = file {
		log::trace!(
			"request_id: {} - Uploading files to s3 server",
			request_id
		);
		upload_static_site_files_to_s3(
			file,
			&hex::encode(&static_site_id),
			config,
			request_id,
		)
		.await?;
	}

	log::trace!("request_id: {} - starting the static site", request_id);
	log::trace!(
		"request_id: {} - getting static site data from db",
		request_id
	);
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let config = config.clone();

	deploy_static_site(
		connection,
		&static_site_id,
		static_site.domain_name.as_deref(),
		&config,
		request_id,
	)
	.await?;

	db::update_static_site_status(
		connection,
		static_site_id,
		&DeploymentStatus::Errored,
	)
	.await?;
	Ok(())
}

pub async fn stop_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Stopping the static site with id: {} and request_id: {}",
		hex::encode(&static_site_id),
		request_id
	);
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let patr_domain = format!("{}.patr.cloud", hex::encode(static_site_id));
	log::trace!("request_id: {} - logging into the ssh server for stopping the static site", request_id);
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	let mut sftp = session.sftp();

	log::trace!(
		"request_id: {} - checking for patr domain's certificate",
		request_id
	);
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

	log::trace!("request_id: {} - updating nginx config. Changing root location to be stopped", request_id);
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

	if let Some(custom_domain) = static_site.domain_name {
		log::trace!(
			"request_id: {} - checking if certificate exists for custom domain",
			request_id
		);
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
		log::trace!(
			"updating nginx config. Changing root location to be stopped"
		);
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
	log::trace!(
		"request_id: {} - static site stopped successfully",
		request_id
	);
	log::trace!("request_id: {} - session closed", request_id);
	log::trace!("request_id: {} - updating db status to stopped", request_id);
	db::update_static_site_status(
		connection,
		static_site_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	Ok(())
}

pub async fn delete_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	service::stop_static_site(connection, static_site_id, config).await?;

	db::update_static_site_name(
		connection,
		static_site_id,
		&format!(
			"patr-deleted: {}-{}",
			static_site.name,
			hex::encode(static_site_id)
		),
	)
	.await?;

	db::update_static_site_status(
		connection,
		static_site_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

	let patr_domain = format!("{}.patr.cloud", hex::encode(static_site_id));
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

	if let Some(domain_name) = static_site.domain_name {
		db::begin_deferred_constraints(connection).await?;

		db::set_domain_name_for_static_site(
			connection,
			static_site_id,
			Some(format!(
				"deleted.patr.cloud.{}.{}",
				hex::encode(static_site_id),
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

pub async fn set_domain_for_static_site_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
	static_site_id: &[u8],
	new_domain_name: Option<&str>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Set domain for static site with id: {} and request_id: {}",
		hex::encode(&static_site_id),
		request_id
	);
	log::trace!(
		"request_id: {} - getting static site info from database",
		request_id
	);
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	let old_domain = static_site.domain_name;

	log::trace!("request_id: {} - logging into the ssh server for adding a new domain name for static site", request_id);
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	log::trace!(
		"request_id: {} - updating database with new domain",
		request_id
	);
	db::begin_deferred_constraints(connection).await?;

	db::set_domain_name_for_static_site(
		connection,
		static_site_id,
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
				update_nginx_for_static_site_with_https(
					new_domain,
					&hex::encode(static_site_id),
					config,
					request_id,
				)
				.await?;
			} else {
				log::trace!("request_id: {} - certificate not present updating nginx with http", request_id);
				update_nginx_for_static_site_with_http(
					new_domain,
					&hex::encode(static_site_id),
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
					update_nginx_for_static_site_with_https(
						new_domain,
						&hex::encode(static_site_id),
						config,
						request_id,
					)
					.await?;
				} else {
					log::trace!(
						"certificate creation failed updating nginx with http"
					);
					update_nginx_for_static_site_with_http(
						new_domain,
						&hex::encode(static_site_id),
						config,
						request_id,
					)
					.await?;
				}
			}
		}
	}
	session.close().await?;
	log::trace!("request_id: {} - session closed", request_id);
	log::trace!("request_id: {} - domains updated successfully", request_id);

	Ok(())
}

pub async fn get_dns_records_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: Settings,
) -> Result<Vec<CNameRecord>, Error> {
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain_name = static_site
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

	Ok(vec![CNameRecord {
		cname: domain_name,
		value: config.ssh.host_name,
	}])
}

pub async fn upload_files_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	file: &str,
	config: &Settings,
	request_id: Uuid,
) -> Result<(), Error> {
	db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	upload_static_site_files_to_s3(
		file,
		&hex::encode(&static_site_id),
		config,
		request_id,
	)
	.await?;

	Ok(())
}

pub async fn get_static_site_validation_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
) -> Result<bool, Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Getting validation status for static site with id: {} and request_id: {}",
		hex::encode(&static_site_id),
		request_id
	);
	log::trace!("request_id: {} - validating the custom domain", request_id);
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain_name = static_site
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

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
		deployment::create_random_content_for_verification(&session).await?;

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
			update_nginx_for_static_site_with_https(
				&domain_name,
				&hex::encode(static_site_id),
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
		deployment::create_https_certificates_for_domain(
			&domain_name,
			config,
			request_id,
		)
		.await?;
		log::trace!("request_id: {} - updating nginx with https", request_id);
		update_nginx_for_static_site_with_https(
			&domain_name,
			&hex::encode(static_site_id),
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

async fn upload_static_site_files_to_s3(
	file: &str,
	static_site_id_string: &str,
	config: &Settings,
	request_id: Uuid,
) -> Result<(), Error> {
	let file_data = base64::decode(file)?;
	log::trace!(
		"request_id: {} - logging into the s3 for uploading static site files",
		request_id
	);
	// let session = SessionBuilder::default()
	// 	.user(config.ssh.username.clone())
	// 	.port(config.ssh.port)
	// 	.keyfile(&config.ssh.key_file)
	// 	.known_hosts_check(KnownHosts::Add)
	// 	.connect(&config.ssh.host)
	// 	.await?;
	// log::trace!(
	// 	"request_id: {} - successfully logged into the server",
	// 	request_id
	// );

	// let aws_s3_client =
	// 	get_s3_client(&config.digitalocean.region, config.clone())
	// 		.await?;
	// log::trace!("request_id: {} - got the s3 client", request_id);

	let mut file_data = Cursor::new(file_data);

	let archive = ZipFileReader::new(&mut file_data).await;

	let mut archive = match archive {
		Ok(archive) => archive,
		Err(e) => {
			log::error!(
				"request_id: {} - error while reading the archive: {:#?}",
				request_id,
				e
			);
			return Err(Error::empty());
		}
	};

	log::trace!(
		"request_id: {} - archive file successfully read",
		request_id
	);

	let region: s3::Region = match config.s3.region.parse() {
		Ok(region) => region,
		Err(e) => {
			log::error!(
				"request_id: {} - error while parsing the region: {:#?}",
				request_id,
				e
			);
			return Err(Error::empty());
		}
	};

	let credentials = match creds::Credentials::new(
		Some(&config.s3.key),
		Some(&config.s3.secret),
		None,
		None,
		None,
	) {
		Ok(credentials) => credentials,
		Err(e) => {
			log::error!(
				"request_id: {} - error while creating the credentials: {:#?}",
				request_id,
				e
			);
			return Err(Error::empty());
		}
	};

	let storage_config = Storage {
		name: "digitalocean".to_string(),
		region,
		credentials,
		bucket: "patr-staging-storage".to_string(),
		location_supported: true,
	};

	for i in 0..archive.entries().len() {
		// let mut cloned_archive = archive.clone();
		// let cloned_archive = cloned_archive.by_index(i)?;

		// let file = archive.by_index(i)?;

		// // Ensure the file path is safe to use as a Path.
		// // It can't contain NULL bytes
		// // It can't resolve to a path outside the current directory
		// // foo/../bar is fine, foo/../../bar is not.
		// // It can't be an absolute path
		// // This will read well-formed ZIP files correctly, and is resistant
		// to // path-based exploits. It is recommended over
		// ZipFile::mangled_name let file_name = cloned_archive
		// 	.enclosed_name()
		// 	.status(500)
		// 	.body(error!(SERVER_ERROR).to_string())?
		// 	.to_str()
		// 	.status(500)
		// 	.body(error!(SERVER_ERROR).to_string())?;
		// let mut file_bytes = file.bytes();
		// let mut file_vec = Vec::new();

		// while let Some(file_bytes) = file_bytes.next() {
		// 	file_vec.push(file_bytes?);
		// }

		let file_info = match archive.entry_reader(i).await {
			Ok(file_info) => file_info,
			Err(e) => {
				log::error!(
					"request_id: {} - error while reading the archive: {:#?}",
					request_id,
					e
				);
				return Err(Error::empty());
			}
		};

		let file_name = file_info.entry().name().to_string();

		let file_info = match file_info.read_to_end_crc().await {
			Ok(file_info) => file_info,
			Err(e) => {
				log::error!(
					"request_id: {} - error while reading the archive: {:#?}",
					request_id,
					e
				);
				return Err(Error::empty());
			}
		};
		log::trace!("request_id: {} - file_name: {}", request_id, file_name);
		// TODO: change file_name to file.enclosed_name()
		// let upload = aws_s3_client
		// 	.put_object()
		// 	.set_key(Some(format!("{}/{}", static_site_id_string, &file_name)))
		// 	.set_body(Some(file_info.into()))
		// 	.set_acl(input)
		// 	.send()
		// 	.await?;
		let bucket = match s3::Bucket::new(
			&storage_config.bucket,
			storage_config.region.clone(),
			storage_config.credentials.clone(),
		) {
			Ok(bucket) => bucket,
			Err(e) => {
				log::error!(
					"request_id: {} - error while creating the bucket: {:#?}",
					request_id,
					e
				);
				return Err(Error::empty());
			}
		};

		let (_, code) = match bucket
			.put_object(
				&format!("{}/{}", static_site_id_string, &file_name),
				&file_info,
			)
			.await
		{
			Ok(upload) => upload,
			Err(e) => {
				log::error!(
					"request_id: {} - error while uploading the file: {:#?}",
					request_id,
					e
				);
				return Err(Error::empty());
			}
		};
		if code == 200 {
			log::trace!(
				"request_id: {} - file successfully uploaded",
				request_id
			);
		} else {
			log::error!(
				"request_id: {} - error while uploading the file",
				request_id
			);
			return Err(Error::empty());
		}
	}
	log::trace!("request_id: {} - uploaded the files to s3", request_id);

	// let mut sftp = session.sftp();
	// let mut zip_file = sftp
	// 	.write_to(format!(
	// 		"/home/web/static-sites/{}.zip",
	// 		static_site_id_string
	// 	))
	// 	.await?;

	// zip_file.write_all(&file_data).await?;
	// zip_file.close().await?;
	// drop(sftp);
	// log::trace!(
	// 	"request_id: {} - creating directory for static sites",
	// 	request_id
	// );

	// //delete existing directory if present
	// let delete_existing_directory_result = session
	// 	.command("rm")
	// 	.arg("-r")
	// 	.arg("-f")
	// 	.arg(format!("/home/web/static-sites/{}/", static_site_id_string))
	// 	.spawn()?
	// 	.wait()
	// 	.await?;

	// if !delete_existing_directory_result.success() {
	// 	return Err(Error::empty());
	// }
	// let create_directory_result = session
	// 	.command("mkdir")
	// 	.arg("-p")
	// 	.arg(format!("/home/web/static-sites/{}/", static_site_id_string))
	// 	.spawn()?
	// 	.wait()
	// 	.await?;

	// if !create_directory_result.success() {
	// 	return Err(Error::empty());
	// }
	// log::trace!("request_id: {} - unzipping the file", request_id);
	// let unzip_result = session
	// 	.command("unzip")
	// 	.arg("-o")
	// 	.arg(format!(
	// 		"/home/web/static-sites/{}.zip",
	// 		static_site_id_string
	// 	))
	// 	.arg("-d")
	// 	.arg(format!("/home/web/static-sites/{}/", static_site_id_string))
	// 	.status()
	// 	.await?;

	// if !unzip_result.success() {
	// 	return Err(Error::empty());
	// }
	// log::trace!("request_id: {} - unzipping successful", request_id);
	// log::trace!("request_id: {} - deleting the zip file", request_id);
	// let delete_zip_file_result = session
	// 	.command("rm")
	// 	.arg("-r")
	// 	.arg(format!(
	// 		"/home/web/static-sites/{}.zip",
	// 		static_site_id_string
	// 	))
	// 	.spawn()?
	// 	.wait()
	// 	.await?;

	// if !delete_zip_file_result.success() {
	// 	return Err(Error::empty());
	// }
	// session.close().await?;
	// log::trace!("request_id: {} - session closed successfully", request_id);
	Ok(())
}

async fn update_nginx_for_static_site_with_http(
	domain: &str,
	static_site_id_string: &str,
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

	root /home/web/static-sites/{static_site_id_string};
	index index.html index.htm;

	location / {{
		try_files $uri.html $uri $uri/ /index.html /index.htm =404;
	}}

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
				domain = domain,
				static_site_id_string = static_site_id_string
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

async fn deploy_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	custom_domain: Option<&str>,
	config: &Settings,
	request_id: Uuid,
) -> Result<(), Error> {
	// update DNS
	log::trace!("request_id: {} - updating DNS", request_id);
	// super::add_cname_record(
	// 	&hex::encode(static_site_id),
	// 	&config.ssh.host_name,
	// 	config,
	// 	false,
	// )
	// .await?;
	// log::trace!("request_id: {} - DNS Updated", request_id);

	// update_nginx_with_all_domains_for_static_site(
	// 	&hex::encode(static_site_id),
	// 	custom_domain,
	// 	config,
	// 	request_id,
	// )
	// .await?;
	kubernetes::update_static_site(connection, static_site_id, config).await?;
	log::trace!("request_id: {} - updating database status", request_id);
	db::update_static_site_status(
		connection,
		static_site_id,
		&DeploymentStatus::Running,
	)
	.await?;
	log::trace!("request_id: {} - updated database status", request_id);
	Ok(())
}

async fn update_nginx_for_static_site_with_https(
	domain: &str,
	static_site_id_string: &str,
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
	root /home/web/static-sites/{static_site_id_string};

	index index.html index.htm;

	ssl_certificate /etc/letsencrypt/live/{domain}/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/{domain}/privkey.pem;
	
	location / {{
		try_files $uri.html $uri $uri/ /index.html /index.htm =404;
	}}

	include snippets/letsencrypt.conf;
	include snippets/patr-verification.conf;
}}
"#,
				domain = domain,
				static_site_id_string = static_site_id_string
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

async fn update_nginx_with_all_domains_for_static_site(
	static_id_string: &str,
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

	let patr_domain = format!("{}.patr.cloud", static_id_string);

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
		update_nginx_for_static_site_with_https(
			&patr_domain,
			static_id_string,
			config,
			request_id,
		)
		.await?;
	} else {
		log::trace!("request_id: {} - certificate does not exists", request_id);
		update_nginx_for_static_site_with_http(
			&patr_domain,
			static_id_string,
			config,
			request_id,
		)
		.await?;
		deployment::create_https_certificates_for_domain(
			&patr_domain,
			config,
			request_id,
		)
		.await?;
		update_nginx_for_static_site_with_https(
			&patr_domain,
			static_id_string,
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
			update_nginx_for_static_site_with_https(
				domain,
				static_id_string,
				config,
				request_id,
			)
			.await?;
		} else {
			update_nginx_for_static_site_with_http(
				domain,
				static_id_string,
				config,
				request_id,
			)
			.await?;
		}
	}
	session.close().await?;
	log::trace!("request_id: {} - session closed successfully!", request_id);
	Ok(())
}

async fn update_static_site_status(
	static_site_id: &[u8],
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_static_site_status(
		app.database.acquire().await?.deref_mut(),
		static_site_id,
		status,
	)
	.await?;

	Ok(())
}

// async fn get_s3_client(
// 	region: &str,
// 	config: Settings,
// ) -> Result<aws_sdk_s3::Client, Error> {
// 	let s3_region = Region::new(config.digitalocean.spaces_region);
// 	let s3_creds = aws_types::Credentials::from_keys(
// 		&config.digitalocean.spaces_access_key,
// 		&config.digitalocean.spaces_secret_key,
// 		None,
// 	)
// 	.provide_credentials()
// 	.await?;

// 	let s3_creds = SharedCredentialsProvider::new(s3_creds);

// 	let s3_config = aws_types::config::Config::builder()
// 		.credentials_provider(s3_creds)
// 		.region(s3_region)
// 		.build();

// 	Ok(aws_sdk_s3::Client::new(&s3_config))
// }

// fn create_my_credential_provider() -> CredentialsProvider {
// 	let mut provider = StaticProvider::new();
// 	provider.set_credentials(
// 		Credentials::new(
// 			config::get_env("AWS_ACCESS_KEY_ID").unwrap(),
// 			config::get_env("AWS_SECRET_ACCESS_KEY").unwrap(),
// 		),
// 	);
// 	provider
// }
