use axum::{
	body::Body,
	extract::{Path, State},
	http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header::InvalidHeaderValue},
	response::IntoResponse,
};
use futures::TryStreamExt;
use oci_spec::distribution::ErrorCode;
use serde::{Deserialize, Serialize};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use super::super::Error;
use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		get_s3_object_name_for_manifest,
		internal_server_error_response,
	},
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
	/// The digest of the blob
	#[preprocess(trim)]
	referrer: String,
}

/// Handles the `GET /v2/<name>/manifests/<reference>` route. i.e. Pushing a manifest. See [end-7](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#endpoints) for more details
#[axum::debug_handler]
pub(super) async fn handle(
	header: HeaderMap,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
	body: Body,
) -> Result<impl IntoResponse, Error> {
	let path = preprocess_stuff(path)?;

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	// TODO: This SHOULD support tags, but we don't have that rn
	let referrer = path.referrer;
	if !(referrer.starts_with("sha256:") && referrer.len() == 71) {
		return Err(convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::ManifestInvalid,
			"Invalid referrer format".to_string(),
		));
	}

	let content_type = header
		.get("Content-Type")
		.ok_or_else(|| {
			convert_oci_error(
				StatusCode::BAD_REQUEST,
				oci_spec::distribution::ErrorCode::ManifestInvalid,
				"Content-Type header is required".to_string(),
			)
		})?
		.to_str()
		.map_err(internal_server_error_response)?;

	if !(content_type == "application/vnd.oci.image.manifest.v1+json") {
		return Err(convert_oci_error(
			StatusCode::BAD_REQUEST,
			oci_spec::distribution::ErrorCode::ManifestInvalid,
			"Unsupported Content-Type".to_string(),
		));
	}

	let mut body_stream = body
		.into_data_stream()
		.map_err(std::io::Error::other)
		.into_async_read()
		.compat();

	let bucket = get_s3_bucket(state.config.clone())?;
	let s3_key = get_s3_object_name_for_manifest(&referrer);

	let status = bucket
		.put_object_stream_with_content_type(&mut body_stream, s3_key, "application/octet-stream")
		.await
		.map_err(internal_server_error_response)?;

	if !(200..300).contains(&status.status_code()) {
		return Err(convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::ManifestInvalid,
			"Failed to push manifest to S3".to_string(),
		));
	}

	// TODO: This also needs to be completed, but rn I don't have the right
	// column
	// query!(
	//     r#"
	//         INSERT INTO
	//     "#
	// )
	// .execute(&mut *database)
	// .await
	// .map_err(internal_server_error_response)?;

	let headers = [
		(
			HeaderName::from_static("Docker-Distribution-API-Version"),
			Some(String::from("registry/2.0")),
		),
		(
			HeaderName::from_static("Location"),
			Some(format!(
				"https://registry.patr.cloud/v2/{}/{}/manifests/{}",
				path.workspace_id, path.repo_name, referrer
			)),
		),
	]
	.into_iter()
	.filter_map(|(name, value)| value.map(|value| (name, value)))
	.map(|(name, value)| Ok::<_, InvalidHeaderValue>((name, HeaderValue::from_str(&value)?)))
	.collect::<Result<HeaderMap, _>>()
	.map_err(internal_server_error_response)?;

	Ok((StatusCode::OK, headers).into_response())
}
