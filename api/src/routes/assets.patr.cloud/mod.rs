use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Builder as S3Builder};
use axum::{
	Router,
	body::Body,
	extract::{Path, State},
	http::{StatusCode, header},
	response::Response,
	routing::get,
};
use tokio_util::io::ReaderStream;

use crate::prelude::*;

/// Sets up the routes for assets.patr.cloud
// #[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/{*path}", get(serve_asset))
		.with_state(state.clone())
}

/// Handler for GET /*path
///
/// Serves static assets from R2/S3. Assets are addressed by their hashed
/// filenames for cache-busting. Responds with aggressive cache headers since
/// hashed URLs are immutable.
async fn serve_asset(
	State(state): State<AppState>,
	Path(path): Path<String>,
) -> Result<Response, StatusCode> {
	let s3 = S3Client::from_conf(
		S3Builder::new()
			.region(Region::new(state.config.s3.region.clone()))
			.endpoint_url(state.config.s3.endpoint.clone())
			.credentials_provider(
				Credentials::builder()
					.access_key_id(&state.config.s3.key)
					.secret_access_key(&state.config.s3.secret)
					.provider_name("Static")
					.build(),
			)
			.force_path_style(state.config.s3.force_path_style)
			.build(),
	);

	let object = s3
		.get_object()
		.bucket(&state.config.s3.bucket)
		.key(&path)
		.send()
		.await
		.map_err(|err| {
			warn!("Failed to get asset from S3: {err}");
			StatusCode::NOT_FOUND
		})?;

	let content_type = extension_to_content_type(&path);

	Ok(Response::builder()
		.status(StatusCode::OK)
		.header(header::CONTENT_TYPE, content_type)
		.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
		.body(Body::from_stream(ReaderStream::new(
			object.body.into_async_read(),
		)))
		.unwrap())
}

/// Maps a file extension to its MIME content type.
fn extension_to_content_type(path: &str) -> &'static str {
	let Some((_, extension)) = path.rsplit_once('.') else {
		return "application/octet-stream";
	};

	match extension {
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
	}
}
