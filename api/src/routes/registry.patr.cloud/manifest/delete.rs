use axum::{
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		Error,
		get_s3_object_name_for_manifest,
		internal_server_error_response,
	},
	utils::helper::{check_repository, check_workspace, get_s3_bucket, preprocess_stuff},
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
	/// Digest, The Session ID
	digest: String,
}

/// Deletes a manifest from the registry.
/// See [OCI Distribution Specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#deleting-manifests)
#[axum::debug_handler]
pub(super) async fn handle(
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
	trace!("Delete Manifest Called");
	let path = preprocess_stuff(path)?;

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	let repository_name = path.repo_name;
	check_repository(&repository_name, state.clone()).await?;

	let digest = path.digest;

	let bucket = get_s3_bucket(state.config.clone())?;
	let s3_key = get_s3_object_name_for_manifest(&digest);

	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let manifest = query!(
		r#"
        SELECT 
            digest 
        FROM 
            container_registry_manifest
        WHERE 
            digest = $1
        LIMIT 
            1
        "#,
		&digest
	)
	.fetch_optional(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	if manifest.is_none() {
		return Ok((
			StatusCode::NOT_FOUND,
			[
				("Content-Type".to_string(), "application/json".to_string()),
				(
					"Docker-Distribution-Api-Version".to_string(),
					"registry/2.0".to_string(),
				),
			],
		)
			.into_response());
	}

	query!(
		r#"
        DELETE FROM 
            container_registry_tag
        WHERE 
            manifest_digest = $1
        "#,
		&digest
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	query!(
		r#"
        DELETE FROM 
            container_registry_repository_manifest
        WHERE 
            manifest_digest = $1 AND
            repository_id = (
                SELECT id FROM container_registry_repository
                WHERE name = $2 AND workspace_id = $3
            )
        "#,
		&digest,
		&repository_name,
		&workspace_id as _
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	query!(
		r#"
        DELETE FROM 
            container_registry_manifest
        WHERE 
            digest = $1
        "#,
		&digest
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	bucket
		.delete_object(&s3_key)
		.await
		.map_err(internal_server_error_response)?;

	Ok((
		StatusCode::ACCEPTED,
		[
			("Content-Type".to_string(), "application/json".to_string()),
			(
				"Docker-Distribution-Api-Version".to_string(),
				"registry/2.0".to_string(),
			),
		],
	)
		.into_response())
}
