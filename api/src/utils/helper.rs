use std::fmt::Display;

use axum::{Json, http::StatusCode};
use oci_spec::distribution::{ErrorCode, ErrorInfoBuilder, ErrorResponse, ErrorResponseBuilder};
use preprocess::Preprocessable;
use s3::Bucket;

use crate::{prelude::*, utils::config::AppConfig};

type Error = (StatusCode, Json<ErrorResponse>);
fn internal_server_error_response(error: impl Display) -> Error {
	error!("{error}");
	(
		StatusCode::INTERNAL_SERVER_ERROR,
		Json(ErrorResponseBuilder::default().errors([]).build().unwrap()),
	)
}

/// Helper function to get the required header value from headers object
pub fn get_header(headers: &axum::http::HeaderMap, key: &str) -> Result<String, Error> {
	let header_value = headers
		.get(key)
		.ok_or_else(|| {
			convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::BlobUploadInvalid,
				format!("{} header is required", key),
			)
		})?
		.to_str()
		.map_err(internal_server_error_response)?;

	Ok(header_value.to_string())
}

/// Create an OCI Error to return
pub fn convert_oci_error(status: StatusCode, oci_code: ErrorCode, message: String) -> Error {
	return (
		status,
		Json(
			ErrorResponseBuilder::default()
				.errors([ErrorInfoBuilder::default()
					.code(oci_code)
					.message(message)
					.detail("".to_string())
					.build()
					.unwrap()])
				.build()
				.unwrap(),
		),
	);
}

/// Get The s3 bucket object
pub fn get_s3_bucket(config: AppConfig) -> Result<Box<Bucket>, Error> {
	Bucket::new(
		config.s3.bucket.as_str(),
		s3::Region::Custom {
			region: config.s3.region,
			endpoint: config.s3.endpoint,
		},
		{
			s3::creds::Credentials::new(
				Some(&config.s3.key),
				Some(&config.s3.secret),
				None,
				None,
				None,
			)
			.map_err(internal_server_error_response)?
		},
	)
	.map_err(internal_server_error_response)
}

/// Preprocess the request, can preprocess stuff like path, query, body
pub fn preprocess_stuff<T>(data: T) -> Result<T::Processed, Error>
where
	T: Preprocessable + Send,
{
	let Ok(process_data) = data.preprocess().inspect_err(|err| {
		error!("Failed to preprocess data: {}", err);
	}) else {
		return Err(convert_oci_error(
			StatusCode::NOT_FOUND,
			ErrorCode::BlobUnknown,
			"Invalid repository name".to_string(),
		));
	};
	Ok(process_data)
}

/// Check if the given workspace exists
pub async fn check_workspace(workspace_id: Uuid, app_state: AppState) -> Result<(), Error> {
	let mut tx = app_state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let row = query!(
		r#"
		SELECT
			*
		FROM
			workspace
		WHERE
			id = $1 AND
			deleted IS NULL
		"#,
		workspace_id as _
	)
	.fetch_optional(&mut *tx)
	.await
	.map_err(internal_server_error_response)?;

	let Some(_) = row else {
		warn!("Workspace {workspace_id} not found");
		return Err(convert_oci_error(
			StatusCode::NOT_FOUND,
			ErrorCode::BlobUnknown,
			"Invalid repository name".to_string(),
		));
	};

	Ok(())
}
