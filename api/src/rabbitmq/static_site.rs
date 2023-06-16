use std::{
	io::{Cursor, Read},
	sync::Arc,
};

use api_models::models::workspace::infrastructure::deployment::DeploymentStatus;
use base64::prelude::*;
use chrono::Utc;
use futures::{stream, StreamExt};
use s3::{creds::Credentials, Bucket, Region};
use zip::ZipArchive;

use crate::{
	db,
	models::rabbitmq::StaticSiteData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: StaticSiteData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		StaticSiteData::CreateStaticSiteUpload {
			static_site_id,
			upload_id,
			file,
			files_length,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Checking if static site: {} if present for the upload: {}",
				request_id,
				static_site_id,
				upload_id
			);

			let static_site =
				db::get_static_site_by_id(&mut *connection, &static_site_id)
					.await?;

			if static_site.is_none() {
				log::trace!(
					"request_id: {} - unable to find any static site with ID: {} for upload: {}",
					request_id,
					static_site_id,
					upload_id
				);
				return Ok(());
			}

			log::trace!(
				"request_id: {} - logging into the s3 for uploading static site files",
				request_id
			);
			let bucket = Arc::new(
				Bucket::new(
					&config.s3.bucket,
					Region::Custom {
						endpoint: config.s3.endpoint.clone(),
						region: config.s3.region.clone(),
					},
					Credentials::new(
						Some(&config.s3.key),
						Some(&config.s3.secret),
						None,
						None,
						None,
					)
					.map_err(|err| {
						log::error!(
							"request_id: {} - error creating credentials: {}",
							request_id,
							err
						);
						Error::empty()
					})?,
				)
				.map_err(|err| {
					log::error!(
						"request_id: {} - error creating bucket: {}",
						request_id,
						err
					);
					Error::empty()
				})?,
			);
			log::trace!("request_id: {} - got the s3 client", request_id);

			db::update_current_live_upload_for_static_site(
				connection,
				&static_site_id,
				&upload_id,
			)
			.await?;

			let file_data = Cursor::new(BASE64_STANDARD.decode(file)?);

			let mut archive = ZipArchive::new(file_data).map_err(|err| {
				log::error!(
					"request_id: {} - error while reading the archive: {:#?}",
					request_id,
					err
				);
				err
			})?;

			log::trace!(
				"request_id: {} - archive file successfully read",
				request_id
			);

			let mut files_vec = Vec::with_capacity(files_length);

			let mut file_size = 0;

			for i in 0..archive.len() {
				let mut file = archive.by_index(i).map_err(|err| {
					log::error!(
						"request_id: {} - error while reading the archive: {}",
						request_id,
						err
					);
					err
				})?;

				file_size += file.size();

				// For now restricting user to upload file of size 100mb max
				if file_size > 100000000 {
					log::error!(
						concat!(
							"request_id: {} - ",
							"Cannot upload in RabbitMQ since it's > 100mb"
						),
						request_id
					);
					log::error!(
						concat!(
							"request_id: {} - ",
							"Ideally shouldn't have reached RabbitMQ at all. ",
							"Ignoring for now"
						),
						request_id
					);
					return Ok(());
				}

				let file_name = if let Some(path) = file.enclosed_name() {
					path.to_string_lossy().to_string()
				} else {
					continue;
				};

				let mut file_content = Vec::with_capacity(file.size() as usize);

				file.read_to_end(&mut file_content).map_err(|err| {
					log::error!(
						"request_id: {} - error while reading the archive: {}",
						request_id,
						err
					);
					err
				})?;

				files_vec.push((file_name.clone(), file_content));
			}

			let length = files_vec.len();
			stream::iter(files_vec)
				.enumerate()
				.map(|(idx, (file_name, file_content))| {
					let bucket = bucket.clone();
					let request_id = request_id.clone();
					log::trace!(
						"request_id: {} uploading file : {}/{}",
						request_id,
						idx + 1,
						length
					);
					let full_file_name = format!(
						"{}/{}/{}",
						static_site_id, upload_id, file_name
					);
					async move {
						let file_extension =
							file_name.split('.').last().unwrap_or("");

						let mime_string =
							get_mime_type_from_file_name(file_extension);

						let code = bucket
							.put_object_with_content_type(
								full_file_name,
								&file_content,
								mime_string,
							)
							.await
							.map_err(|err| {
								log::error!(
									"request_id: {} - S3 upload error: {}",
									request_id,
									err
								);
								Error::empty()
							})?
							.status_code();

						if !(200..300).contains(&code) {
							log::error!(
								"request_id: {} - S3 upload error: {}",
								request_id,
								code
							);
							return Err(Error::empty());
						}
						Ok(())
					}
				})
				.buffer_unordered(num_cpus::get() * 4)
				.collect::<Vec<Result<_, _>>>()
				.await
				.into_iter()
				.collect::<Result<(), _>>()
				.map_err(|err| {
					log::error!(
						concat!(
							"request_id: {} - ",
							"Error while uploading file to S3: {}"
						),
						request_id,
						err.get_error()
					);
					err
				})?;

			db::set_static_site_upload_as_processed(
				&mut *connection,
				&static_site_id,
				&upload_id,
				Some(&Utc::now()),
			)
			.await?;

			log::trace!(
				"request_id: {} - Updating the static site and db status",
				request_id
			);

			if let Some(static_site) = static_site {
				if static_site.status == DeploymentStatus::Stopped {
					log::trace!(
						concat!(
							"Static site with ID: {} is stopped manully,",
							" skipping update static site k8s api call"
						),
						static_site_id
					);
				} else {
					service::update_cloudflare_running_upload(
						&mut *connection,
						&static_site_id,
						&upload_id,
						config,
						&request_id,
					)
					.await?;
				}
			}

			Ok(())
		}
	}
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
