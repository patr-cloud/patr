use axum::{
	body::Body,
	extract::{Path, State},
	http::{HeaderMap, StatusCode},
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
	utils::helper::{
		Referrer,
		check_repository,
		check_workspace,
		convert_oci_error,
		get_header,
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
	/// The digest/tag of the blob
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
	trace!("PUT called on get manifest");
	let path = preprocess_stuff(path)?;

	let repository_name = path.repo_name;
	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;
	let repository_id = check_repository(&repository_name, state.clone()).await?;

	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let content_type = get_header(&header, "Content-Type")?;

	let referrer = get_referrer(&path.referrer);
	let manifest = match referrer {
		Referrer::Digest(digest) => query!(
			r#"
				SELECT
					mani.digest,
					mani.size
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
		.map(|mani| (mani.digest, mani.size)),
		Referrer::Tag(tag) => query!(
			r#"
				SELECT 
					mani.digest,
					mani.size 
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
		.map(|mani| (mani.digest, mani.size)),
	};

	let (digest, size) = manifest.ok_or_else(|| {
		convert_oci_error(
			StatusCode::NOT_FOUND,
			ErrorCode::ManifestUnknown,
			"Manifest not found".to_string(),
		)
	})?;

	let mut body_stream = body
		.into_data_stream()
		.map_err(std::io::Error::other)
		.into_async_read()
		.compat();

	let bucket = get_s3_bucket(state.config.clone())?;
	let s3_key = get_s3_object_name_for_manifest(&digest);

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

	query!(
		r#"
		INSERT INTO container_registry_manifest(
			digest,
			size,
			created_at,
			content_type
		) VALUES (
		 	$1,
			$2,
			NOW(),
			$3
		);
		"#,
		digest,
		size as _,
		content_type
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	query!(
		r#"
		INSERT INTO container_registry_repository_manifest(
			repository_id,
			manifest_digest,
			created_at
		) VALUES (
			$1,
			$2,
			NOW()
		);
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	database
		.commit()
		.await
		.map_err(internal_server_error_response)?;

	let headers = [
		("Docker-Distribution-API-Version", "registry/2.0"),
		(
			"Location",
			&format!(
				"/v2/{}/{}/manifests/{}",
				path.workspace_id, repository_name, &digest
			),
		),
	];

	Ok((StatusCode::OK, headers).into_response())
}
