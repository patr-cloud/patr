use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Builder as S3Builder};
use axum::{
	Router,
	body::Body,
	extract::State,
	http::{StatusCode, Uri, header},
	response::Response,
	routing::get,
};
use tokio_util::io::ReaderStream;

use crate::prelude::*;

/// Sets up the routes for assets.patr.cloud
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/email/images/{*path}", get(serve_asset))
		.route("/error-pages/{*path}", get(serve_asset))
		.with_state(state.clone())
}

/// Serves static assets from R2/S3. Only paths mounted above are reachable.
async fn serve_asset(State(state): State<AppState>, uri: Uri) -> Result<Response, StatusCode> {
	let key = uri.path().trim_start_matches('/');

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
		.key(key)
		.send()
		.await
		.map_err(|err| {
			warn!("Failed to get asset from S3: {err}");
			StatusCode::NOT_FOUND
		})?;

	let ext = key.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin");
	let content_type = mime_guess::from_ext(ext)
		.first_or_octet_stream()
		.to_string();

	Ok(Response::builder()
		.status(StatusCode::OK)
		.header(header::CONTENT_TYPE, content_type)
		.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
		.body(Body::from_stream(ReaderStream::new(
			object.body.into_async_read(),
		)))
		.unwrap())
}
