use std::{io::Cursor, net::Ipv4Addr};

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use async_zip::read::seek::ZipFileReader;
use aws_config::RetryConfig;
use aws_sdk_s3::{model::ObjectCannedAcl, Endpoint, Region};
use aws_types::credentials::{ProvideCredentials, SharedCredentialsProvider};
use cloudflare::{
	endpoints::{
		dns::{
			CreateDnsRecord,
			CreateDnsRecordParams,
			DeleteDnsRecord,
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
use http::Uri;

use crate::{
	db,
	error,
	models::rbac,
	service::{
		deployment::{kubernetes},
	},
	utils::{get_current_time_millis, settings::Settings, validator, Error},
	Database,
};

pub async fn create_static_site_deployment_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	domain_name: Option<&str>,
	user_id: &Uuid,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	// validate static site name
	log::trace!("request_id: {} - validating static site name", request_id);
	if !validator::is_deployment_name_valid(name) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_DEPLOYMENT_NAME).to_string())?;
	}

	log::trace!(
		"request_id: {} - validating static site domain name",
		request_id
	);
	if let Some(domain_name) = domain_name {
		let is_god_user = user_id == rbac::GOD_USER_ID.get().unwrap();
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

	let static_site_id = db::generate_new_resource_id(connection).await?;
	log::trace!("request_id: {} - creating static site resource", request_id);
	db::create_resource(
		connection,
		&static_site_id,
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

	log::trace!("request_id: {} - Adding entry to database", request_id);
	db::create_static_site(
		connection,
		&static_site_id,
		name,
		domain_name,
		workspace_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - static site created successfully",
		request_id
	);
	Ok(static_site_id)
}

pub async fn start_static_site_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
	config: &Settings,
	file: Option<&str>,
	request_id: &Uuid,
) -> Result<(), Error> {
	let _ = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"Deploying the static site with id: {} and request_id: {}",
		static_site_id,
		request_id
	);

	log::trace!("Updating DNS records");
	add_a_record(
		static_site_id.as_str(),
		config.ssh.host.parse::<Ipv4Addr>()?,
		config,
		false,
	)
	.await?;
	log::trace!("DNS records updated");

	if let Some(file) = file {
		log::trace!(
			"request_id: {} - Uploading files to s3 server",
			request_id
		);
		upload_static_site_files_to_s3(
			connection,
			file,
			static_site_id,
			config,
			&request_id,
		)
		.await?;
	}

	log::trace!("request_id: {} - starting the static site", request_id);

	let result = kubernetes::update_static_site(
		connection,
		static_site_id,
		config,
		request_id,
	)
	.await;

	match result {
		Ok(()) => {
			log::trace!(
				"request_id: {} - updating database status",
				request_id
			);
			db::update_static_site_status(
				connection,
				static_site_id,
				&DeploymentStatus::Running,
			)
			.await?;
			log::trace!("request_id: {} - updated database status", request_id);
		}
		Err(e) => {
			db::update_static_site_status(
				connection,
				static_site_id,
				&DeploymentStatus::Errored,
			)
			.await?;
			log::error!(
				"Error occured during deployment of static site: {}",
				e.get_error()
			);
		}
	}
	Ok(())
}

pub async fn stop_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let _ = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	kubernetes::delete_static_site_from_k8s(
		connection,
		static_site_id,
		config,
		request_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - static site stopped successfully",
		request_id
	);
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
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let patr_domain = format!("{}.patr.cloud", static_site_id);

	kubernetes::delete_static_site_from_k8s(
		connection,
		static_site_id,
		config,
		request_id,
	)
	.await?;

	db::update_static_site_name(
		connection,
		static_site_id,
		&format!("patr-deleted: {}-{}", static_site.name, static_site_id),
	)
	.await?;

	kubernetes::delete_tls_certificate(
		connection,
		static_site_id,
		config,
		request_id,
	)
	.await?;

	db::update_static_site_status(
		connection,
		static_site_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

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
				name: Some("patr.cloud".to_string()),
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
			if let DnsContent::A { .. } = record.content {
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

	Ok(())
}

pub async fn set_domain_for_static_site_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
	static_site_id: &Uuid,
	new_domain_name: Option<&str>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!(
		"Set domain for static site with id: {} and request_id: {}",
		static_site_id,
		request_id
	);
	log::trace!(
		"request_id: {} - getting static site info from database",
		request_id
	);
	let _ = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

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

	kubernetes::update_static_site(
		connection,
		static_site_id,
		config,
		&request_id,
	)
	.await?;
	log::trace!("request_id: {} - domains updated successfully", request_id);

	Ok(())
}

pub async fn get_dns_records_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &Uuid,
	config: Settings,
) -> Result<Vec<(String, String)>, Error> {
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain_name = static_site
		.domain_name
		.status(400)
		.body(error!(INVALID_DOMAIN_NAME).to_string())?;

	Ok(vec![(domain_name, config.ssh.host_name)])
}

pub async fn upload_static_site_files_to_s3(
	connection: &mut <Database as sqlx::Database>::Connection,
	file: &str,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - getting static site details from db",
		request_id
	);
	db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	let file_data = base64::decode(file)?;
	log::trace!(
		"request_id: {} - logging into the s3 for uploading static site files",
		request_id
	);
	let aws_s3_client = get_s3_client(config.clone()).await?;
	log::trace!("request_id: {} - got the s3 client", request_id);

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

	for i in 0..archive.entries().len() {
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
		log::trace!(
			"request_id: {} - file_name: {}/{}",
			request_id,
			static_site_id,
			file_name
		);
		// TODO: change file_name to file.enclosed_name()
		let file_extension = file_name
			.split('.')
			.last()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
		let mime_string = get_mime_type_from_file_name(file_extension);

		let _ = aws_s3_client
			.put_object()
			.bucket(config.s3.bucket.clone())
			.key(format!("{}/{}", static_site_id, file_name))
			.body(file_info.into())
			.acl(ObjectCannedAcl::PublicRead)
			.content_type(mime_string)
			.send()
			.await?;
	}
	log::trace!("request_id: {} - uploaded the files to s3", request_id);

	Ok(())
}

async fn get_s3_client(config: Settings) -> Result<aws_sdk_s3::Client, Error> {
	let s3_region = Region::new(config.s3.region.to_string());
	let s3_creds = aws_types::Credentials::from_keys(
		config.s3.key.to_string(),
		config.s3.secret.to_string(),
		None,
	)
	.provide_credentials()
	.await?;

	let s3_creds = SharedCredentialsProvider::new(s3_creds);

	let shared_config = aws_config::Config::builder()
		.credentials_provider(s3_creds)
		.region(s3_region)
		.retry_config(RetryConfig::disabled())
		.build();

	let s3_endpoint =
		format!("https://{}", config.s3.endpoint).parse::<Uri>()?;

	let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
		.retry_config(RetryConfig::disabled())
		.endpoint_resolver(Endpoint::immutable(s3_endpoint))
		.build();

	Ok(aws_sdk_s3::Client::from_conf(s3_config))
}

async fn add_a_record(
	sub_domain: &str,
	target: Ipv4Addr,
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
				name: Some("patr.cloud".to_string()),
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
	let expected_dns_record = DnsContent::A { content: target };
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
		if let DnsContent::A { .. } = record.content {
			record.name == full_domain
		} else {
			false
		}
	});
	if let Some(record) = dns_record {
		if let DnsContent::A { content } = record.content {
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

fn get_mime_type_from_file_name(file_extension: &str) -> &str {
	match file_extension {
		"html" => "text/html",
		"htm" => "text/html",
		"shtml" => "text/html",
		"xhtml" => "application/xhtml+xml",
		"css" => "text/css",
		"xml" => "text/xml",
		"atom" => "application/atom+xml",
		"rss" => "application/rss+xml",
		"js" => "application/javascript",
		"mml" => "text/mathml",
		"png" => "image/png",
		"jpg" => "image/jpeg",
		"jpeg" => "image/jpeg",
		"gif" => "image/gif",
		"ico" => "image/x-icon",
		"svg" => "image/svg+xml",
		"svgz" => "image/svg+xml",
		"tif" => "image/tiff",
		"tiff" => "image/tiff",
		"json" => "application/json",
		"pdf" => "application/pdf",
		"txt" => "text/plain",
		"mp4" => "video/mp4",
		"webm" => "video/webm",
		"mp3" => "audio/mpeg",
		"ogg" => "audio/ogg",
		"wav" => "audio/wav",
		"woff" => "application/font-woff",
		"woff2" => "application/font-woff2",
		"ttf" => "application/font-truetype",
		"otf" => "application/font-opentype",
		"eot" => "application/vnd.ms-fontobject",
		"mpg" => "video/mpeg",
		"mpeg" => "video/mpeg",
		"mov" => "video/quicktime",
		"avi" => "video/x-msvideo",
		"flv" => "video/x-flv",
		"m4v" => "video/x-m4v",
		"jad" => "text/vnd.sun.j2me.app-descriptor",
		"wml" => "text/vnd.wap.wml",
		"htc" => "text/x-component",
		"avif" => "image/avif",
		"webp" => "image/webp",
		"wbmp" => "image/vnd.wap.wbmp",
		"jng" => "image/x-jng",
		"bmp" => "image/x-ms-bmp",
		"jar" => "application/java-archive",
		"war" => "application/java-archive",
		"ear" => "application/java-archive",
		"hqx" => "application/mac-binhex40",
		"doc" => "application/msword",
		"ps" => "application/postscript",
		"eps" => "application/postscript",
		"ai" => "application/postscript",
		"rtf" => "application/rtf",
		"m3u8" => "application/vnd.apple.mpegurl",
		"kml" => "application/vnd.google-earth.kml+xml",
		"kmz" => "application/vnd.google-earth.kmz",
		"xls" => "application/vnd.ms-excel",
		"ppt" => "application/vnd.ms-powerpoint",
		"odg" => "application/vnd.oasis.opendocument.graphics",
		"odp" => "application/vnd.oasis.opendocument.presentation",
		"ods" => "application/vnd.oasis.opendocument.spreadsheet",
		"odt" => "application/vnd.oasis.opendocument.text",
		"pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
		"xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
		"docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
		"wmlc" => "application/vnd.wap.wmlc",
		"wasm" => "application/wasm",
		"7z" => "application/x-7z-compressed",
		"cco" => "application/x-cocoa",
		"jardiff" => "application/x-java-archive-diff",
		"jnlp" => "application/x-java-jnlp-file",
		"run" => "application/x-makeself",
		"pl" => "application/x-perl",
		"pm" => "application/x-perl",
		"prc" => "application/x-pilot",
		"pdb" => "application/x-pilot",
		"rar" => "application/x-rar-compressed",
		"rpm" => "application/x-redhat-package-manager",
		"sea" => "application/x-sea",
		"swf" => "application/x-shockwave-flash",
		"sit" => "application/x-stuffit",
		"tcl" => "application/x-tcl",
		"tk" => "application/x-tcl",
		"der" => "application/x-x509-ca-cert",
		"pem" => "application/x-x509-ca-cert",
		"crt" => "application/x-x509-ca-cert",
		"xpi" => "application/x-xpinstall",
		"xspf" => "application/xspf+xml",
		"zip" => "application/zip",
		"bin" => "application/octet-stream",
		"exe" => "application/octet-stream",
		"dll" => "application/octet-stream",
		"deb" => "application/octet-stream",
		"dmg" => "application/octet-stream",
		"iso" => "application/octet-stream",
		"img" => "application/octet-stream",
		"msi" => "application/octet-stream",
		"msp" => "application/octet-stream",
		"msm" => "application/octet-stream",
		"mid" => "audio/midi",
		"midi" => "audio/midi",
		"kar" => "audio/midi",
		"m4a" => "audio/x-m4a",
		"ra" => "audio/x-realaudio",
		"3gpp" => "video/3gpp",
		"3gp" => "video/3gpp",
		"ts" => "video/mp2t",
		"mng" => "video/x-mng",
		"asx" => "video/x-ms-asf",
		"asf" => "video/x-ms-asf",
		"wmv" => "video/x-ms-wmv",
		_ => "application/octet-stream",
	}
}
