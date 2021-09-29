use std::{ops::DerefMut, time::Duration};

use eve_rs::AsError;
use openssh::{KnownHosts, SessionBuilder};
use tokio::{io::AsyncWriteExt, task, time};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{CNameRecord, DeploymentStatus},
		rbac,
	},
	service::{
		self,
		deployment::{
			add_cname_record,
			create_https_certificates_for_domain,
			create_random_content_for_verification,
		},
	},
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub async fn create_static_site_deployment_in_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
	name: &str,
	domain_name: Option<&str>,
	file: &str,
	config: &Settings,
) -> Result<Uuid, Error> {
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
		organisation_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("adding entry to database");
	db::create_static_site(
		connection,
		static_site_id,
		name,
		domain_name,
		organisation_id,
	)
	.await?;

	// Deploy the app as soon as it's created, so that any existing images can
	// be deployed
	let static_site_id = static_site_id.to_vec();
	let file = file.to_string();
	let config = config.clone();
	task::spawn(async move {
		time::sleep(Duration::from_millis(1000)).await;

		log::trace!("uploading files to nginx server");
		let upload_result = upload_static_site_files_to_nginx(
			&file,
			&hex::encode(&static_site_id),
			&config,
		)
		.await;
		if let Err(error) = upload_result {
			let _ = update_static_site_status(
				&static_site_id,
				&DeploymentStatus::Errored,
			)
			.await;
			log::info!("Error during file upload, {}", error.get_error());
		}

		let result =
			start_static_site_deployment(&static_site_id, &config).await;

		if let Err(error) = result {
			let _ = update_static_site_status(
				&static_site_id,
				&DeploymentStatus::Errored,
			)
			.await;
			log::info!(
				"Error with the static site deployment, {}",
				error.get_error()
			);
		}
	});

	Ok(static_uuid)
}

pub async fn start_static_site_deployment(
	static_site_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("starting the static site");
	let app = service::get_app();
	log::trace!("getting static site data from db");
	let static_site = db::get_static_site_deployment_by_id(
		app.database.acquire().await?.deref_mut(),
		static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// update DNS
	log::trace!("updating DNS");
	add_cname_record(
		&hex::encode(static_site_id),
		"nginx.patr.cloud",
		config,
		false,
	)
	.await?;
	log::trace!("DNS Updated");

	update_nginx_with_all_domains_for_static_site(
		&hex::encode(static_site_id),
		static_site.domain_name.as_deref(),
		config,
	)
	.await?;

	db::update_static_site_status(
		app.database.acquire().await?.deref_mut(),
		static_site_id,
		&DeploymentStatus::Running,
	)
	.await?;
	Ok(())
}

pub async fn stop_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
) -> Result<(), Error> {
	log::trace!("Getting deployment id from db");
	let static_site =
		db::get_static_site_deployment_by_id(connection, static_site_id)
			.await?
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let patr_domain = format!("{}.patr.cloud", hex::encode(static_site_id));
	log::trace!("logging into the ssh server for stopping the static site");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	let mut sftp = session.sftp();

	log::trace!("checking for patr domain's certificate");
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

	log::trace!("updating nginx config. Changing root location to be stopped");
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
		log::trace!("checking if certificate exists for custom domain");
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

	log::trace!("reloaded nginx");

	session.close().await?;
	log::trace!("static site stopped successfully");
	log::trace!("session closed");
	log::trace!("updating db status to stopped");
	db::update_static_site_status(
		connection,
		static_site_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	Ok(())
}

pub async fn set_domain_for_static_site_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
	static_site_id: &[u8],
	new_domain_name: Option<&str>,
) -> Result<(), Error> {
	log::trace!("getting static site info from database");
	let static_site =
		db::get_static_site_deployment_by_id(connection, static_site_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let old_domain = static_site.domain_name;

	log::trace!("logging into the ssh server for adding a new domain name for static site");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;

	log::trace!("updating database with new domain");
	db::set_domain_name_for_static_site(
		connection,
		static_site_id,
		new_domain_name,
	)
	.await?;

	match (new_domain_name, old_domain.as_deref()) {
		(None, None) => {
			log::trace!("both domains are null");
		} // Do nothing
		(Some(new_domain), None) => {
			log::trace!("old domain null, adding new domain");
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
				)
				.await?;
			} else {
				log::trace!("certificate not present updating nginx with http");
				update_nginx_for_static_site_with_http(
					new_domain,
					&hex::encode(static_site_id),
					config,
				)
				.await?;
			}
		}
		(None, Some(domain_name)) => {
			log::trace!("new domain null, deleting old domain");
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
			log::trace!("replacing old domain with new domain");
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
					log::trace!("certificate creation successfull updating nginx with https");
					update_nginx_for_static_site_with_https(
						new_domain,
						&hex::encode(static_site_id),
						config,
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
					)
					.await?;
				}
			}
		}
	}
	session.close().await?;
	log::trace!("session closed");
	log::trace!("domains updated successfully");

	Ok(())
}

pub async fn get_dns_records_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
) -> Result<Vec<CNameRecord>, Error> {
	let deployment =
		db::get_static_site_deployment_by_id(connection, static_site_id)
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

pub async fn get_static_site_validation_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	config: &Settings,
) -> Result<bool, Error> {
	log::trace!("validating the custom domain");
	let deployment =
		db::get_static_site_deployment_by_id(connection, static_site_id)
			.await?
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain_name = deployment
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

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
			update_nginx_for_static_site_with_https(
				&domain_name,
				&hex::encode(static_site_id),
				config,
			)
			.await?;
			return Ok(true);
		}
		log::trace!("certificate does not exist creating a new one");
		create_https_certificates_for_domain(&domain_name, config).await?;
		log::trace!("updating nginx with https");
		update_nginx_for_static_site_with_https(
			&domain_name,
			&hex::encode(static_site_id),
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

async fn upload_static_site_files_to_nginx(
	file: &str,
	static_site_id_string: &str,
	config: &Settings,
) -> Result<(), Error> {
	let file_data = base64::decode(file)?;
	log::trace!("logging into the ssh server for uploading static site files");
	let session = SessionBuilder::default()
		.user(config.ssh.username.clone())
		.port(config.ssh.port)
		.keyfile(&config.ssh.key_file)
		.known_hosts_check(KnownHosts::Add)
		.connect(&config.ssh.host)
		.await?;
	log::trace!("successfully logged into the server");

	let mut sftp = session.sftp();
	let mut zip_file = sftp
		.write_to(format!("/home/{}.zip", static_site_id_string))
		.await?;

	zip_file.write_all(&file_data).await?;
	zip_file.close().await?;
	drop(sftp);
	log::trace!("creating directory for static sites");
	let create_directory_result = session
		.command("mkdir")
		.arg(format!("/home/web/static-sites/{}/", static_site_id_string))
		.spawn()?
		.wait()
		.await?;

	if !create_directory_result.success() {
		return Err(Error::empty());
	}
	log::trace!("unzipping the file");
	let unzip_result = session
		.command("unzip")
		.arg(format!("/home/{}.zip", static_site_id_string))
		.arg("-d")
		.arg(format!("/home/web/static-sites/{}/", static_site_id_string))
		.status()
		.await?;

	if !unzip_result.success() {
		return Err(Error::empty());
	}
	log::trace!("unzipping successful");
	log::trace!("deleting the zip file");
	let delete_zip_file_result = session
		.command("rm")
		.arg("-r")
		.arg(format!("/home/{}.zip", static_site_id_string))
		.spawn()?
		.wait()
		.await?;

	if !delete_zip_file_result.success() {
		return Err(Error::empty());
	}
	session.close().await?;
	log::trace!("session closed successfully");
	Ok(())
}

async fn update_nginx_for_static_site_with_http(
	domain: &str,
	static_site_id_string: &str,
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

	root /home/web/static-sites/{static_site_id_string};

	index index.html index.htm;

	location / {{
		try_files $uri.html $uri $uri/ =404;
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

async fn update_nginx_for_static_site_with_https(
	domain: &str,
	static_site_id_string: &str,
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

	root /home/web/static-sites/{static_site_id_string};

	index index.html index.htm;

	ssl_certificate /etc/letsencrypt/live/{domain}/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/{domain}/privkey.pem;
	
	location / {{
		try_files $uri.html $uri $uri/ =404;
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
	log::trace!("updated sites-enabled for https");
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

async fn update_nginx_with_all_domains_for_static_site(
	static_id_string: &str,
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

	let patr_domain = format!("{}.patr.cloud", static_id_string);

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
		update_nginx_for_static_site_with_https(
			&patr_domain,
			static_id_string,
			config,
		)
		.await?;
	} else {
		log::trace!("certificate does not exists");
		update_nginx_for_static_site_with_http(
			&patr_domain,
			static_id_string,
			config,
		)
		.await?;
		create_https_certificates_for_domain(&patr_domain, config).await?;
		update_nginx_for_static_site_with_https(
			&patr_domain,
			static_id_string,
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
			update_nginx_for_static_site_with_https(
				domain,
				static_id_string,
				config,
			)
			.await?;
		} else {
			update_nginx_for_static_site_with_http(
				domain,
				static_id_string,
				config,
			)
			.await?;
		}
	}

	session.close().await?;
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
