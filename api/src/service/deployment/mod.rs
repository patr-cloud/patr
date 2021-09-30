mod aws;
#[allow(clippy::module_inception)]
mod deployment;
mod digitalocean;
mod managed_database;
mod static_site;

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
use openssh::{KnownHosts, Session, SessionBuilder};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use tokio::io::AsyncWriteExt;

pub use self::{deployment::*, managed_database::*, static_site::*};
use crate::{
	db,
	error,
	models::db_mapping::{
		CNameRecord,
		DeploymentRequestMethod,
		DeploymentRequestProtocol,
		IpResponse,
	},
	utils::{settings::Settings, Error},
	Database,
};

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
