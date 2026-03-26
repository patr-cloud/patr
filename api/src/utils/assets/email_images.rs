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

			let content_type = mime_guess::from_ext(ext)
				.first_or_octet_stream()
				.to_string();

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
