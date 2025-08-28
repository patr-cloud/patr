use axum::{
	Json,
	body::Body,
	extract::{Path, State},
	http::{
		HeaderMap,
		HeaderName,
		HeaderValue,
		Method,
		StatusCode,
		header::{self, InvalidHeaderValue},
	},
	response::IntoResponse,
};
use oci_spec::distribution::{ErrorCode, ErrorInfoBuilder, ErrorResponseBuilder};
use preprocess::Preprocessable;
use s3::Bucket;
use serde::{Deserialize, Serialize};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		Error,
		get_s3_object_name_for_blob,
		internal_server_error_response,
	},
	utils::helper::check_workspace,
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
	/// The digest of the blob
	#[preprocess(lowercase, trim)]
	digest: String,
}

/// Defines the `GET` and `HEAD` routes for `/blobs/<digest>`. [end-2](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#endpoints)
#[axum::debug_handler]
pub(super) async fn handle(
	method: Method,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
	let Ok(path) = path.preprocess() else {
		return Err((
			StatusCode::NOT_FOUND,
			Json(
				ErrorResponseBuilder::default()
					.errors([ErrorInfoBuilder::default()
						.code(ErrorCode::BlobUnknown)
						.message("Invalid repository name".to_string())
						.detail("".to_string())
						.build()
						.unwrap()])
					.build()
					.unwrap(),
			),
		));
	};

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	let bucket = Bucket::new(
		state.config.s3.bucket.as_str(),
		s3::Region::Custom {
			region: state.config.s3.region,
			endpoint: state.config.s3.endpoint,
		},
		{
			s3::creds::Credentials::new(
				Some(&state.config.s3.key),
				Some(&state.config.s3.secret),
				None,
				None,
				None,
			)
			.map_err(internal_server_error_response)?
		},
	)
	.map_err(internal_server_error_response)?;

	let s3_key = get_s3_object_name_for_blob(&path.digest);
	let (head, _) = bucket
		.head_object(&s3_key)
		.await
		.map_err(internal_server_error_response)?;

	let headers = [
		(
			HeaderName::from_static("Docker-Distribution-API-Version"),
			Some(String::from("registry/2.0")),
		),
		(
			HeaderName::from_static("Docker-Content-Digest"),
			Some(path.digest.to_string()),
		),
		(header::ACCEPT_RANGES, head.accept_ranges),
		(header::CACHE_CONTROL, head.cache_control),
		(header::CONTENT_DISPOSITION, head.content_disposition),
		(header::CONTENT_ENCODING, head.content_encoding),
		(header::CONTENT_LANGUAGE, head.content_language),
		(
			header::CONTENT_LENGTH,
			head.content_length.map(|length| length.to_string()),
		),
		(header::CONTENT_TYPE, head.content_type),
		(header::ETAG, head.e_tag),
		(header::EXPIRES, head.expires),
		(header::LAST_MODIFIED, head.last_modified),
	]
	.into_iter()
	.filter_map(|(name, value)| value.map(|value| (name, value)))
	.map(|(name, value)| Ok::<_, InvalidHeaderValue>((name, HeaderValue::from_str(&value)?)))
	.collect::<Result<HeaderMap, _>>()
	.map_err(internal_server_error_response)?;

	if matches!(method, Method::HEAD) {
		// HEAD request. head the blob from S3 and set the headers
		Ok((StatusCode::OK, headers).into_response())
	} else {
		// GET request. return the blob from S3
		let object = bucket
			.get_object_stream(&s3_key)
			.await
			.map_err(internal_server_error_response)?;
		if !(200..300).contains(&object.status_code) {
			return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
		}
		Ok((StatusCode::OK, headers, Body::from_stream(object.bytes)).into_response())
	}
}
