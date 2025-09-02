use axum::{
	body::{Body, to_bytes},
	extract::{Path, State},
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
};
use futures::TryStreamExt;
use oci_spec::distribution::ErrorCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
	reference: String,
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

	let referrer = get_referrer(&path.reference);

	let body_bytes = to_bytes(body, usize::MAX)
		.await
		.inspect(|body| {
			trace!("body chunk size: {}", body.len());
		})
		.inspect_err(|error| {
			error!("Error reading body stream: {}", error);
		})
		.map_err(internal_server_error_response)?;

	let size = body_bytes.len();
	let body_stream = body_bytes.to_vec();

	let digest = match referrer {
		Referrer::Tag(tag) => {
			let digest = format!("sha256:{:x}", Sha256::digest(&body_bytes));
			// Check if tag exists
			let tag_in_db = query!(
				r#"
				SELECT 
					*
				FROM
					container_registry_tag AS tag
				WHERE
					repository_id = $1 AND
					name = $2;
				"#,
				repository_id as _,
				tag
			)
			.fetch_optional(&mut *database)
			.await
			.map_err(internal_server_error_response)?;

			if tag_in_db.is_none() {
				query!(
					r#"
					INSERT INTO
						container_registry_tag(
							repository_id,
							name,
							manifest_digest
						) VALUES (
							$1,
							$2,
							$3
						);
					"#,
					repository_id as _,
					tag,
					digest
				)
				.execute(&mut *database)
				.await
				.map_err(internal_server_error_response)?;
			}

			digest
		}
		Referrer::Digest(digest) => digest,
	};

	let bucket = get_s3_bucket(state.config.clone())?;
	let s3_key = get_s3_object_name_for_manifest(&digest);
	let status = bucket
		.put_object(s3_key, &body_stream)
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
		size as i32,
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

	let canonical_digest = format!("sha256:{:x}", Sha256::digest(&body_bytes));
	trace!("digest: {canonical_digest}");
	let headers = [
		("Docker-Distribution-API-Version", "registry/2.0"),
		(
			"Location",
			&format!(
				"/v2/{}/{}/manifests/{}",
				path.workspace_id, repository_name, &digest
			),
		),
		("Docker-Content-Digest", &canonical_digest),
	];

	Ok((StatusCode::CREATED, headers).into_response())
}
