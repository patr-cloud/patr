use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Builder as S3Builder, primitives::ByteStream};
use futures::{StreamExt, stream};

use crate::{prelude::*, utils::config::S3Config};

/// Embedded email image assets. These are compiled into the binary at build
/// time and uploaded to R2 at startup.
#[derive(rust_embed::RustEmbed)]
#[folder = "../assets/emails/images/"]
pub struct EmailImages;

#[askama::filter_fn]
/// Askama custom filter that converts an image filename into its hashed
/// `assets.patr.cloud` URL. The hash is derived from the file's SHA-256 at
/// compile time, ensuring cache-busting when images change.
///
/// Usage in templates: `{{ "header.png"|asset_url }}`
pub fn asset_url(filename: &str, _env: &dyn askama::Values) -> askama::Result<String> {
	let file = EmailImages::get(filename).ok_or(askama::Error::Fmt)?;
	let hash = hex::encode(file.metadata.sha256_hash());
	let ext = filename.rsplit('.').next().unwrap_or("bin");
	Ok(format!(
		"{}/email/images/{hash}.{ext}",
		if cfg!(debug_assertions) {
			"http://localhost:3004"
		} else {
			"https://assets.patr.cloud"
		}
	))
}

/// Uploads all embedded email images to R2/S3 at startup.
///
/// For each image, the SHA-256 hash is computed from the embedded content and
/// used as the S3 key (with appropriate extension). If an object with that key
/// already exists, the upload is skipped. Uploads run in parallel.
pub async fn upload_email_images(s3_config: &S3Config) {
	if cfg!(debug_assertions) {
		info!("Running in debug mode, skipping email image upload");
		return;
	}

	let s3 = S3Client::from_conf(
		S3Builder::new()
			.region(Region::new(s3_config.region.clone()))
			.endpoint_url(s3_config.endpoint.clone())
			.credentials_provider(
				Credentials::builder()
					.access_key_id(&s3_config.key)
					.secret_access_key(&s3_config.secret)
					.provider_name("Static")
					.build(),
			)
			.force_path_style(s3_config.force_path_style)
			.build(),
	);

	stream::iter(EmailImages::iter())
		.for_each_concurrent(4, async |filename| {
			let Some(file) = EmailImages::get(&filename) else {
				warn!("Embedded file not found: {filename}");
				return;
			};

			let hash = hex::encode(file.metadata.sha256_hash());
			let ext = filename.rsplit('.').next().unwrap_or("bin");
			let key = format!("email/images/{hash}.{ext}");

			// Check if already uploaded
			let exists = s3
				.head_object()
				.bucket(&s3_config.bucket)
				.key(&key)
				.send()
				.await
				.is_ok();

			if exists {
				debug!("Asset already uploaded, skipping: {filename} -> {key}");
				return;
			}

			let content_type = match ext {
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
				"pptx" => concat!(
					"application/vnd.openxmlformats",
					"-officedocument.presentationml.presentation"
				),
				"xlsx" => concat!(
					"application/vnd.openxmlformats",
					"-officedocument.spreadsheetml.sheet"
				),
				"docx" => concat!(
					"application/vnd.openxmlformats",
					"-officedocument.wordprocessingml.document"
				),
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
			};

			match s3
				.put_object()
				.bucket(&s3_config.bucket)
				.key(&key)
				.content_type(content_type)
				.body(ByteStream::from(file.data.to_vec()))
				.send()
				.await
			{
				Ok(_) => info!("Uploaded asset: {filename} -> {key}"),
				Err(err) => error!("Failed to upload asset {filename}: {err}"),
			}
		})
		.await;

	info!("Email image upload complete");
}
