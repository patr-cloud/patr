use axum::{
	body::Body,
	extract::{Path, State},
	http::{HeaderMap, Method, Response, StatusCode},
	response::IntoResponse,
};
use oci_spec::distribution::ErrorCode;
use serde::{Deserialize, Serialize};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		Error,
		get_s3_object_name_for_manifest,
		internal_server_error_response,
	},
	utils::helper::{
		Referrer,
		check_repository,
		check_workspace,
		convert_oci_error,
		get_referrer,
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
	#[preprocess(trim)]
	reference: String,
}

/// Handles the `HEAD /v2/<name>/manifests/<reference>` route. [`end-3`](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pulling-blobs)
#[axum::debug_handler]
pub(super) async fn handle(
	header: HeaderMap,
	method: Method,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
	trace!("HEAD/GET called on get manifest");
	let path = preprocess_stuff(path)?;
	trace!("Headers: {:#?}", header);

	let repository_name = path.repo_name;
	check_repository(&repository_name, state.clone()).await?;
	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	let referrer = get_referrer(&path.reference);
	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let manifest = match referrer {
		Referrer::Digest(digest) => query!(
			r#"
			SELECT
				mani.digest,
				mani.size,
				mani.content_type
			FROM
				container_registry_manifest AS mani
			WHERE
				digest = $1;
			"#,
			digest
		)
		.fetch_optional(&mut *database)
		.await
		.map_err(internal_server_error_response)?
		.map(|mani| (mani.digest, mani.size, mani.content_type)),
		Referrer::Tag(tag) => query!(
			r#"
			SELECT 
				mani.digest,
				mani.size,
				mani.content_type
			FROM 
				container_registry_manifest AS mani
			INNER JOIN
				container_registry_tag AS tag
			ON
				mani.digest = tag.manifest_digest
			WHERE
				tag.name = $1;
			"#,
			tag
		)
		.fetch_optional(&mut *database)
		.await
		.map_err(internal_server_error_response)?
		.map(|mani| (mani.digest, mani.size, mani.content_type)),
	};

	let (digest, size, content_type) = manifest.ok_or_else(|| {
		convert_oci_error(
			StatusCode::NOT_FOUND,
			ErrorCode::ManifestUnknown,
			"Manifest not found".to_string(),
		)
	})?;

	if matches!(method, Method::HEAD) {
		// HEAD request. just set the headers
		return Ok((
			StatusCode::OK,
			[
				("Docker-Distribution-API-Version", "registry/2.0"),
				("Docker-Content-Digest", &digest),
				("Content-Type", &content_type),
				("Content-Length", &size.to_string()),
			],
		)
			.into_response());
	} else {
		let bucket = get_s3_bucket(state.config.clone())?;
		let s3_key = get_s3_object_name_for_manifest(&digest);
		let object = bucket
			.get_object(&s3_key)
			.await
			.map_err(internal_server_error_response)?;

		if !(200..300).contains(&object.status_code()) {
			return Err(convert_oci_error(
				StatusCode::NOT_FOUND,
				ErrorCode::BlobUnknown,
				"Blob not found".to_string(),
			));
		}

		return Ok(Response::builder()
			.status(StatusCode::OK)
			.header("Docker-Distribution-API-Version", "registry/2.0")
			.header("Docker-Content-Digest", &digest)
			.header("Content-Type", &content_type)
			.header("Content-Length", &size.to_string())
			.body(Body::from(object.into_bytes()))
			.map_err(internal_server_error_response)?);
	}
}
