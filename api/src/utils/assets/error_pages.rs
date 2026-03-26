use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Builder as S3Builder, primitives::ByteStream};
use futures::{StreamExt, stream};

use crate::{prelude::*, utils::config::S3Config};

/// Embedded ingress error page assets. These are compiled into the binary at
/// build time and uploaded to R2 at startup.
#[derive(rust_embed::RustEmbed)]
#[folder = "../assets/error-pages/"]
pub struct ErrorPageAssets;

const S3_PREFIX: &str = "error-pages";

/// Uploads all embedded error page assets to R2/S3 at startup.
pub async fn upload_error_pages(s3_config: &S3Config) {
	if cfg!(debug_assertions) {
		info!("Running in debug mode, skipping error page upload");
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

	stream::iter(ErrorPageAssets::iter())
		.for_each_concurrent(4, async |filename| {
			let Some(file) = ErrorPageAssets::get(&filename) else {
				warn!("Embedded file not found: {filename}");
				return;
			};

			let ext = match filename.rsplit_once('.') {
				Some((_, e)) => e,
				None => "bin",
			};
			let key = format!("{S3_PREFIX}/{filename}");

			let exists = s3
				.head_object()
				.bucket(&s3_config.bucket)
				.key(&key)
				.send()
				.await
				.is_ok();

			if exists {
				debug!("Error page asset already uploaded, skipping: {filename}");
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
				Ok(_) => info!("Uploaded error page asset: {filename}"),
				Err(err) => error!("Failed to upload error page asset {filename}: {err}"),
			}
		})
		.await;

	info!("Error page upload complete");
}
