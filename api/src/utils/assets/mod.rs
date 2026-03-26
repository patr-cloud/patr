use super::config::S3Config;

/// Embedded email image assets — uploaded to R2 at startup, referenced in
/// email templates via the `asset_url` Askama filter.
pub mod email_images;
/// Embedded ingress error page assets — uploaded to R2 at startup, served by
/// the ingress worker when users hit error states.
pub mod error_pages;

/// Uploads all embedded assets (email images and error pages) to R2/S3.
/// Called once at startup.
pub async fn initialize(s3_config: &S3Config) {
	email_images::upload_email_images(s3_config).await;
	error_pages::upload_error_pages(s3_config).await;
}
