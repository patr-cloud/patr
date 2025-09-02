use axum::{
	body::Body,
	extract::{Path, State},
	http::{Method, StatusCode},
	response::IntoResponse,
};
use oci_spec::distribution::ErrorCode;
use serde::{Deserialize, Serialize};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		Error,
		get_s3_object_name_for_blob,
		internal_server_error_response,
	},
	utils::helper::{
		check_repository,
		check_workspace,
		convert_oci_error,
		get_s3_bucket,
		preprocess_stuff,
	},
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
	trace!("GET/HEAD called on get blob");
	let path = preprocess_stuff(path)?;

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	let repository_name = path.repo_name;
	check_repository(&repository_name, state.clone()).await?;

	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;
	info!("Database Initiated");
	let bucket = get_s3_bucket(state.config.clone())?;
	info!("s3 bucket Initiated");

	let s3_key = get_s3_object_name_for_blob(&path.digest);

	let size = query!(
		r#"
		SELECT
			size
		FROM
			container_registry_layer_blob
		WHERE
			digest = $1
		"#,
		&path.digest
	)
	.fetch_optional(&mut *database)
	.await
	.map_err(internal_server_error_response)?
	.map(|rec| rec.size);

	let size = size.ok_or_else(|| {
		convert_oci_error(
			StatusCode::NOT_FOUND,
			ErrorCode::ManifestUnknown,
			"Manifest not found".to_string(),
		)
	})?;

	let headers = [
		("Docker-Distribution-API-Version", "registry/2.0"),
		("Docker-Content-Digest", &path.digest),
		("Content-Length", &size.to_string()),
	];

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
