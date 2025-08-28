use axum::{
	Json,
	body::Body,
	extract::{Path, Query, State},
	http::{HeaderMap, HeaderName, StatusCode},
	response::IntoResponse,
};
use oci_spec::distribution::{ErrorCode, ErrorInfoBuilder, ErrorResponseBuilder};
use serde::{Deserialize, Serialize};
use tokio_util::io::StreamReader;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{Error, internal_server_error_response},
	utils::helper::{check_workspace, convert_oci_error, get_s3_bucket, preprocess_stuff},
};

#[preprocess::sync]
/// The parameters that are passed in the path of the request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathParams {
	/// The workspace ID of the repository
	workspace_id: Uuid,
	/// The name of the repository
	#[preprocess(regex = r"[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*")]
	repo_name: String,
}

#[preprocess::sync]
/// The Query Parameters that are passed
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct QueryParams {
	/// The Digest of the blob
	#[preprocess(lowercase, trim)]
	digest: String,
}

/// Defines the `POST` route for `/blobs/uploads`. [end-4a](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#endpoints) and [end-4b](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#endpoints)
#[axum::debug_handler]
pub(super) async fn handle(
	header: HeaderMap,
	Path(path): Path<PathParams>,
	Query(query): Query<QueryParams>,
	State(state): State<AppState>,
	body: Body,
) -> Result<impl IntoResponse, Error> {
	let path = preprocess_stuff(path)?;
	let query = preprocess_stuff(query)?;

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	// let digest = query.digest;

	// if !digest.to_string() {
	// 	todo!("Create this later")
	// }

	let header_length = header
		.get("Content-Length")
		.ok_or_else(|| {
			convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::BlobUploadInvalid,
				"Content-Length header is required".to_string(),
			)
		})?
		.to_str()
		.map_err(internal_server_error_response)?;
	let header_content_type = header
		.get("Content-Type")
		.ok_or_else(|| {
			convert_oci_error(
				StatusCode::UNSUPPORTED_MEDIA_TYPE,
				ErrorCode::BlobUploadInvalid,
				"Content-Type header is required".to_string(),
			)
		})?
		.to_str()
		.map_err(internal_server_error_response)?;

	if header_content_type != "application/octet-stream" {
		return Err::<(), _>(convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::BlobUploadInvalid,
			format!(
				"Content-Type must be application/octet-stream, got {}",
				header_content_type
			),
		));
	}

	let body_stream = body.into_data_stream();
	let bucket = get_s3_bucket(state.config.clone())?;

	let mut reader = StreamReader::new(body_stream);
	let status = bucket
		.put_object_stream_with_content_type(reader, "/", "application/octet-stream")
		.await;

	let headers = [
		(
			HeaderName::from_static("Docker-Distribution-API-Version"),
			Some(String::from("registry/2.0")),
		),
		(
			HeaderName::from_static("Docker-Content-Digest"),
			Some(query.digest.to_string()),
		),
	];

	Ok((
		StatusCode::OK,
		[],
		Json(
			ErrorResponseBuilder::default()
				.errors([ErrorInfoBuilder::default()
					.code(ErrorCode::BlobUploadInvalid)
					.message("".to_string())
					.detail("".to_string())
					.build()
					.unwrap()])
				.build()
				.unwrap(),
		),
	))
}
