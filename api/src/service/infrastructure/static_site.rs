use std::io::{Cursor, Read};

use api_models::{
	models::workspace::infrastructure::{
		deployment::DeploymentStatus,
		static_site::StaticSiteDetails,
	},
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use eve_rs::AsError;
use s3::{creds::Credentials, Bucket, Region};
use zip::ZipArchive;

use crate::{
	db::{self, StaticSitePlan},
	error,
	models::rbac,
	service::{self, infrastructure::kubernetes},
	utils::{settings::Settings, validator, Error},
	Database,
};

pub async fn create_static_site_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	file: Option<String>,
	message: &str,
	uploaded_by: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	// validate static site name
	log::trace!("request_id: {} - validating static site name", request_id);
	if !validator::is_static_site_name_valid(name) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_STATIC_SITE_NAME).to_string())?;
	}

	log::trace!(
		"request_id: {} - validating static site domain name",
		request_id
	);

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

	log::trace!("request_id: {} - Checking resource limit", request_id);
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	log::trace!("request_id: {} - Checking static site limit", request_id);
	if static_site_limit_crossed(connection, workspace_id, request_id).await? {
		return Error::as_result()
			.status(400)
			.body(error!(STATIC_SITE_LIMIT_EXCEEDED).to_string())?;
	}

	let creation_time = Utc::now();
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
		&creation_time,
	)
	.await?;

	log::trace!("request_id: {} - Adding entry to database", request_id);
	db::create_static_site(connection, &static_site_id, name, workspace_id)
		.await?;

	let static_site_plan =
		match db::get_static_sites_for_workspace(connection, workspace_id)
			.await?
			.len()
		{
			(0..=3) => StaticSitePlan::Free,
			(4..=25) => StaticSitePlan::Pro,
			(26..) => StaticSitePlan::Unlimited,
			_ => unreachable!(),
		};

	db::update_static_site_usage_history(
		connection,
		workspace_id,
		&static_site_plan,
		&creation_time,
	)
	.await?;

	log::trace!(
		"request_id: {} - static site created successfully",
		request_id
	);

	if let Some(file) = file {
		log::trace!(
			"request_id: {} - Creating Static-site upload resource",
			request_id
		);
		let upload_id = db::generate_new_resource_id(connection).await?;
		log::trace!("request_id: {} - Creating upload history", request_id);

		db::create_resource(
			connection,
			&upload_id,
			&format!("Static site upload: {}", upload_id),
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::STATIC_SITE_UPLOAD)
				.unwrap(),
			workspace_id,
			&creation_time,
		)
		.await?;

		db::create_static_site_upload_history(
			connection,
			&upload_id,
			&static_site_id,
			message,
			uploaded_by,
			&Utc::now(),
		)
		.await?;
		db::update_current_live_upload_for_static_site(
			connection,
			&static_site_id,
			&upload_id,
		)
		.await?;

		log::trace!("request_id: {} - Starting static site", request_id);

		db::update_static_site_status(
			connection,
			&static_site_id,
			&DeploymentStatus::Deploying,
		)
		.await?;

		update_static_site_and_db_status(
			connection,
			workspace_id,
			&static_site_id,
			Some(&file),
			&upload_id,
			&StaticSiteDetails {},
			config,
			request_id,
		)
		.await?;
	}

	Ok(static_site_id)
}

pub async fn update_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	name: Option<&str>,
	file: Option<String>,
	message: &str,
	uploaded_by: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Option<Uuid>, Error> {
	log::trace!("request_id: {} - getting static site details", request_id);

	if let Some(name) = name {
		db::update_static_site_name(connection, static_site_id, name).await?;
	}

	let upload_id = if let Some(file) = file {
		let upload_id = db::generate_new_resource_id(connection).await?;
		let now = Utc::now();

		db::create_resource(
			connection,
			&upload_id,
			&format!("Static site upload: {}", upload_id),
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::STATIC_SITE_UPLOAD)
				.unwrap(),
			workspace_id,
			&now,
		)
		.await?;

		log::trace!("request_id: {} - Creating upload history", request_id);
		db::create_static_site_upload_history(
			connection,
			&upload_id,
			static_site_id,
			message,
			uploaded_by,
			&now,
		)
		.await?;
		db::update_current_live_upload_for_static_site(
			connection,
			static_site_id,
			&upload_id,
		)
		.await?;

		log::trace!("request_id: {} - Uploading the static site", request_id);
		service::upload_static_site_files_to_s3(
			connection,
			&file,
			static_site_id,
			&upload_id,
			config,
			request_id,
		)
		.await?;

		log::trace!(
			"request_id: {} - Creating Static-site upload resource",
			request_id
		);

		Some(upload_id)
	} else {
		None
	};

	Ok(upload_id)
}

pub async fn stop_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - Getting deployment id from db", request_id);

	kubernetes::delete_kubernetes_static_site(
		workspace_id,
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
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	kubernetes::delete_kubernetes_static_site(
		workspace_id,
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

	db::update_static_site_status(
		connection,
		static_site_id,
		&DeploymentStatus::Deleted,
	)
	.await?;

	let static_site_plan = match db::get_static_sites_for_workspace(
		connection,
		&static_site.workspace_id,
	)
	.await?
	.len()
	{
		(0..=3) => StaticSitePlan::Free,
		(4..=25) => StaticSitePlan::Pro,
		(26..) => StaticSitePlan::Unlimited,
		_ => unreachable!(),
	};

	db::update_static_site_usage_history(
		connection,
		&static_site.workspace_id,
		&static_site_plan,
		&DateTime::from(Utc::now()),
	)
	.await?;

	Ok(())
}

pub async fn upload_static_site_files_to_s3(
	connection: &mut <Database as sqlx::Database>::Connection,
	file: &str,
	static_site_id: &Uuid,
	upload_id: &Uuid,
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
	let bucket = Bucket::new(
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
	})?;
	log::trace!("request_id: {} - got the s3 client", request_id);

	let file_data = Cursor::new(file_data);

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

	let mut files_vec = Vec::new();

	for i in 0..archive.len() {
		let mut file = archive.by_index(i).map_err(|err| {
			log::error!(
				"request_id: {} - error while reading the archive: {:#?}",
				request_id,
				err
			);
			err
		})?;

		let file_name = if let Some(path) = file.enclosed_name() {
			path.to_string_lossy().to_string()
		} else {
			continue;
		};

		let mut file_content = Vec::with_capacity(file.size() as usize);

		file.read_to_end(&mut file_content).map_err(|err| {
			log::error!(
				"request_id: {} - error while reading the archive: {:#?}",
				request_id,
				err
			);
			err
		})?;

		log::trace!(
			"request_id: {} - file_name: {}/{}/{}",
			request_id,
			static_site_id,
			upload_id,
			file_name
		);

		let file_extension = file_name.split('.').last().unwrap_or("");

		let mime_string = get_mime_type_from_file_name(file_extension);

		files_vec.push((
			file_name.clone(),
			file_content,
			mime_string.to_string(),
		));
	}

	for (file_name, file_content, mime_string) in files_vec {
		let code = bucket
			.put_object_with_content_type(
				format!("{}/{}/{}", static_site_id, upload_id, file_name),
				&file_content,
				&mime_string,
			)
			.await
			.map_err(|err| {
				log::error!(
					"request_id: {} - error pushing static site file to S3: {}",
					request_id,
					err
				);
				Error::empty()
			})?
			.status_code();

		if !(200..300).contains(&code) {
			log::error!(
				"request_id: {} - error pushing static site file to S3: {}",
				request_id,
				code
			);
			return Err(Error::empty());
		}
	}
	log::trace!("request_id: {} - uploaded the files to s3", request_id);

	Ok(())
}

pub async fn update_static_site_and_db_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	file: Option<&str>,
	upload_id: &Uuid,
	_running_details: &StaticSiteDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating kubernetes static site",
		request_id
	);
	let result = service::update_kubernetes_static_site(
		workspace_id,
		static_site_id,
		upload_id,
		&StaticSiteDetails {},
		config,
		request_id,
	)
	.await;

	if let Some(file) = file {
		log::trace!(
			"request_id: {} - Uploading static site files to S3",
			request_id
		);
		service::upload_static_site_files_to_s3(
			connection,
			file,
			static_site_id,
			upload_id,
			config,
			request_id,
		)
		.await?;
	}

	if let Err(err) = result {
		log::error!(
			"request_id: {} - Error occured while deploying site `{}`: {}",
			request_id,
			static_site_id,
			err.get_error()
		);
		// TODO log in audit log that there was an error while deploying
		db::update_static_site_status(
			connection,
			static_site_id,
			&DeploymentStatus::Errored,
		)
		.await?;

		db::update_current_live_upload_for_static_site(
			connection,
			static_site_id,
			upload_id,
		)
		.await?;

		Err(err)
	} else {
		db::update_static_site_status(
			connection,
			static_site_id,
			&DeploymentStatus::Running,
		)
		.await?;

		db::update_current_live_upload_for_static_site(
			connection,
			static_site_id,
			upload_id,
		)
		.await?;

		Ok(())
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

async fn static_site_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	request_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!(
		"request_id: {} - Checking if free limits are crossed",
		request_id
	);

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let current_static_sites =
		db::get_all_secrets_in_workspace(connection, workspace_id)
			.await?
			.len();

	log::trace!(
		"request_id: {} - Checking if static site limits are crossed",
		request_id
	);
	if current_static_sites + 1 > workspace.static_site_limit as usize {
		return Ok(true);
	}

	Ok(false)
}
