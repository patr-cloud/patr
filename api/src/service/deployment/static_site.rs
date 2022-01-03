use std::{io::Cursor, net::Ipv4Addr};

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
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{CNameRecord, DeploymentStatus},
		rbac,
	},
	service::deployment::kubernetes,
	utils::{get_current_time_millis, settings::Settings, validator, Error},
	Database,
};

pub async fn create_static_site_deployment_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
	name: &str,
	domain_name: Option<&str>,
	user_id: &[u8],
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

	log::trace!("request_id: {} - creating static site resource", request_id);
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

	log::trace!("request_id: {} - Adding entry to database", request_id);
	db::create_static_site(
		connection,
		static_site_id,
		name,
		domain_name,
		workspace_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - static site created successfully",
		request_id
	);
	Ok(static_uuid)
}

pub async fn start_static_site_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
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
		hex::encode(&static_site_id),
		request_id
	);

	log::trace!("Updating DNS records");
	add_a_record(
		&hex::encode(static_site_id),
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
			request_id,
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
	static_site_id: &[u8],
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let _ = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	kubernetes::delete_static_site(
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
	static_site_id: &[u8],
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let patr_domain = format!("{}.patr.cloud", hex::encode(static_site_id));

	kubernetes::delete_static_site(
		connection,
		static_site_id,
		config,
		request_id,
	)
	.await?;

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

pub async fn upload_static_site_files_to_s3(
	connection: &mut <Database as sqlx::Database>::Connection,
	file: &str,
	static_site_id: &[u8],
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let static_site_id_string = hex::encode(static_site_id);
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
			static_site_id_string,
			file_name
		);
		// TODO: change file_name to file.enclosed_name()
		let file_extension = file_name
			.split('.')
			.last()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
		let mime_string = get_mime_type_from_file_name(file_extension)?;

		let _ = aws_s3_client
			.put_object()
			.bucket(config.s3.bucket.clone())
			.key(format!("{}/{}", static_site_id_string, file_name))
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

fn get_mime_type_from_file_name(file_extension: &str) -> Result<String, Error> {
	match file_extension {
		"html" => Ok("text/html".to_string()),
		"htm" => Ok("text/html".to_string()),
		"shtml" => Ok("text/html".to_string()),
		"xhtml" => Ok("application/xhtml+xml".to_string()),
		"css" => Ok("text/css".to_string()),
		"xml" => Ok("text/xml".to_string()),
		"atom" => Ok("application/atom+xml".to_string()),
		"rss" => Ok("application/rss+xml".to_string()),
		"js" => Ok("application/javascript".to_string()),
		"mml" => Ok("text/mathml".to_string()),
		"png" => Ok("image/png".to_string()),
		"jpg" => Ok("image/jpeg".to_string()),
		"jpeg" => Ok("image/jpeg".to_string()),
		"gif" => Ok("image/gif".to_string()),
		"ico" => Ok("image/x-icon".to_string()),
		"svg" => Ok("image/svg+xml".to_string()),
		"svgz" => Ok("image/svg+xml".to_string()),
		"tif" => Ok("image/tiff".to_string()),
		"tiff" => Ok("image/tiff".to_string()),
		"json" => Ok("application/json".to_string()),
		"pdf" => Ok("application/pdf".to_string()),
		"txt" => Ok("text/plain".to_string()),
		"mp4" => Ok("video/mp4".to_string()),
		"webm" => Ok("video/webm".to_string()),
		"mp3" => Ok("audio/mpeg".to_string()),
		"ogg" => Ok("audio/ogg".to_string()),
		"wav" => Ok("audio/wav".to_string()),
		"woff" => Ok("application/font-woff".to_string()),
		"woff2" => Ok("application/font-woff2".to_string()),
		"ttf" => Ok("application/font-truetype".to_string()),
		"otf" => Ok("application/font-opentype".to_string()),
		"eot" => Ok("application/vnd.ms-fontobject".to_string()),
		"mpg" => Ok("video/mpeg".to_string()),
		"mpeg" => Ok("video/mpeg".to_string()),
		"mov" => Ok("video/quicktime".to_string()),
		"avi" => Ok("video/x-msvideo".to_string()),
		"flv" => Ok("video/x-flv".to_string()),
		"m4v" => Ok("video/x-m4v".to_string()),
		"jad" => Ok("text/vnd.sun.j2me.app-descriptor".to_string()),
		"wml" => Ok("text/vnd.wap.wml".to_string()),
		"htc" => Ok("text/x-component".to_string()),
		"avif" => Ok("image/avif".to_string()),
		"webp" => Ok("image/webp".to_string()),
		"wbmp" => Ok("image/vnd.wap.wbmp".to_string()),
		"jng" => Ok("image/x-jng".to_string()),
		"bmp" => Ok("image/x-ms-bmp".to_string()),
		"jar" => Ok("application/java-archive".to_string()),
		"war" => Ok("application/java-archive".to_string()),
		"ear" => Ok("application/java-archive".to_string()),
		"hqx" => Ok("application/mac-binhex40".to_string()),
		"doc" => Ok("application/msword".to_string()),
		"ps" => Ok("application/postscript".to_string()),
		"eps" => Ok("application/postscript".to_string()),
		"ai" => Ok("application/postscript".to_string()),
		"rtf" => Ok("application/rtf".to_string()),
		"m3u8" => Ok("application/vnd.apple.mpegurl".to_string()),
		"kml" => Ok("application/vnd.google-earth.kml+xml".to_string()),
		"kmz" => Ok("application/vnd.google-earth.kmz".to_string()),
		"xls" => Ok("application/vnd.ms-excel".to_string()),
		"ppt" => Ok("application/vnd.ms-powerpoint".to_string()),
		"odg" => Ok("application/vnd.oasis.opendocument.graphics".to_string()),
		"odp" => Ok("application/vnd.oasis.opendocument.presentation".to_string()),
		"ods" => Ok("application/vnd.oasis.opendocument.spreadsheet".to_string()),
		"odt" => Ok("application/vnd.oasis.opendocument.text".to_string()),
		"pptx" => Ok("application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string()),
		"xlsx" => Ok("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()),
		"docx" => Ok("application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()),
		"wmlc" => Ok("application/vnd.wap.wmlc".to_string()),
		"wasm" => Ok("application/wasm".to_string()),
		"7z" => Ok("application/x-7z-compressed".to_string()),
		"cco" => Ok("application/x-cocoa".to_string()),
		"jardiff" => Ok("application/x-java-archive-diff".to_string()),
		"jnlp" => Ok("application/x-java-jnlp-file".to_string()),
		"run" => Ok("application/x-makeself".to_string()),
		"pl" => Ok("application/x-perl".to_string()),
		"pm" => Ok("application/x-perl".to_string()),
		"prc" => Ok("application/x-pilot".to_string()),
		"pdb" => Ok("application/x-pilot".to_string()),
		"rar" => Ok("application/x-rar-compressed".to_string()),
		"rpm" => Ok("application/x-redhat-package-manager".to_string()),
		"sea" => Ok("application/x-sea".to_string()),
		"swf" => Ok("application/x-shockwave-flash".to_string()),
		"sit" => Ok("application/x-stuffit".to_string()),
		"tcl" => Ok("application/x-tcl".to_string()),
		"tk" => Ok("application/x-tcl".to_string()),
		"der" => Ok("application/x-x509-ca-cert".to_string()),
		"pem" => Ok("application/x-x509-ca-cert".to_string()),
		"crt" => Ok("application/x-x509-ca-cert".to_string()),
		"xpi" => Ok("application/x-xpinstall".to_string()),
		"xspf" => Ok("application/xspf+xml".to_string()),
		"zip" => Ok("application/zip".to_string()),
		"bin" => Ok("application/octet-stream".to_string()),
		"exe" => Ok("application/octet-stream".to_string()),
		"dll" => Ok("application/octet-stream".to_string()),
		"deb" => Ok("application/octet-stream".to_string()),
		"dmg" => Ok("application/octet-stream".to_string()),
		"iso" => Ok("application/octet-stream".to_string()),
		"img" => Ok("application/octet-stream".to_string()),
		"msi" => Ok("application/octet-stream".to_string()),
		"msp" => Ok("application/octet-stream".to_string()),
		"msm" => Ok("application/octet-stream".to_string()),
		"mid" => Ok("audio/midi".to_string()),
		"midi" => Ok("audio/midi".to_string()),
		"kar" => Ok("audio/midi".to_string()),
		"m4a" => Ok("audio/x-m4a".to_string()),
		"ra" => Ok("audio/x-realaudio".to_string()),
		"3gpp" => Ok("video/3gpp".to_string()),
		"3gp" => Ok("video/3gpp".to_string()),
		"ts" => Ok("video/mp2t".to_string()),
		"mng" => Ok("video/x-mng".to_string()),
		"asx" => Ok("video/x-ms-asf".to_string()),
		"asf" => Ok("video/x-ms-asf".to_string()),
		"wmv" => Ok("video/x-ms-wmv".to_string()),
		_ => Ok("application/octet-stream".to_string()),
	}
	/*
	text/html                                        html htm shtml;
	text/css                                         css;
	text/xml                                         xml;
	image/gif                                        gif;
	image/jpeg                                       jpeg jpg;
	application/javascript                           js;
	application/atom+xml                             atom;
	application/rss+xml                              rss;

	text/mathml                                      mml;
	text/plain                                       txt;
	text/vnd.sun.j2me.app-descriptor                 jad;
	text/vnd.wap.wml                                 wml;
	text/x-component                                 htc;

	image/avif                                       avif;
	image/png                                        png;
	image/svg+xml                                    svg svgz;
	image/tiff                                       tif tiff;
	image/vnd.wap.wbmp                               wbmp;
	image/webp                                       webp;
	image/x-icon                                     ico;
	image/x-jng                                      jng;
	image/x-ms-bmp                                   bmp;

	font/woff                                        woff;
	font/woff2                                       woff2;

	application/java-archive                         jar war ear;
	application/json                                 json;
	application/mac-binhex40                         hqx;
	application/msword                               doc;
	application/pdf                                  pdf;
	application/postscript                           ps eps ai;
	application/rtf                                  rtf;
	application/vnd.apple.mpegurl                    m3u8;
	application/vnd.google-earth.kml+xml             kml;
	application/vnd.google-earth.kmz                 kmz;
	application/vnd.ms-excel                         xls;
	application/vnd.ms-fontobject                    eot;
	application/vnd.ms-powerpoint                    ppt;
	application/vnd.oasis.opendocument.graphics      odg;
	application/vnd.oasis.opendocument.presentation  odp;
	application/vnd.oasis.opendocument.spreadsheet   ods;
	application/vnd.oasis.opendocument.text          odt;
	application/vnd.openxmlformats-officedocument.presentationml.presentation
													 pptx;
	application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
													 xlsx;
	application/vnd.openxmlformats-officedocument.wordprocessingml.document
													 docx;
	application/vnd.wap.wmlc                         wmlc;
	application/wasm                                 wasm;
	application/x-7z-compressed                      7z;
	application/x-cocoa                              cco;
	application/x-java-archive-diff                  jardiff;
	application/x-java-jnlp-file                     jnlp;
	application/x-makeself                           run;
	application/x-perl                               pl pm;
	application/x-pilot                              prc pdb;
	application/x-rar-compressed                     rar;
	application/x-redhat-package-manager             rpm;
	application/x-sea                                sea;
	application/x-shockwave-flash                    swf;
	application/x-stuffit                            sit;
	application/x-tcl                                tcl tk;
	application/x-x509-ca-cert                       der pem crt;
	application/x-xpinstall                          xpi;
	application/xhtml+xml                            xhtml;
	application/xspf+xml                             xspf;
	application/zip                                  zip;

	application/octet-stream                         bin exe dll;
	application/octet-stream                         deb;
	application/octet-stream                         dmg;
	application/octet-stream                         iso img;
	application/octet-stream                         msi msp msm;

	audio/midi                                       mid midi kar;
	audio/mpeg                                       mp3;
	audio/ogg                                        ogg;
	audio/x-m4a                                      m4a;
	audio/x-realaudio                                ra;

	video/3gpp                                       3gpp 3gp;
	video/mp2t                                       ts;
	video/mp4                                        mp4;
	video/mpeg                                       mpeg mpg;
	video/quicktime                                  mov;
	video/webm                                       webm;
	video/x-flv                                      flv;
	video/x-m4v                                      m4v;
	video/x-mng                                      mng;
	video/x-ms-asf                                   asx asf;
	video/x-ms-wmv                                   wmv;
	video/x-msvideo                                  avi;
	*/
}
