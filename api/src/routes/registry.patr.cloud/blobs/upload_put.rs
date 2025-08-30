use axum::{
	body::{Body, HttpBody},
	extract::{Path, Query, State},
	http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
	response::IntoResponse,
};
use oci_spec::distribution::ErrorCode;
use serde::{Deserialize, Serialize};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{Error, internal_server_error_response},
	utils::helper::{check_workspace, convert_oci_error, preprocess_stuff},
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
	/// Reference, The Session ID
	session_id: Uuid,
}

#[preprocess::sync]
/// The Query Parameters that are passed
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct QueryParams {
	/// The Digest of the blob
	#[preprocess(lowercase, trim)]
	#[serde(default)]
	digest: String,
}

/// Handles the `GET /v2/<name>/blobs/uploads/<reference>?digest=<digest>` route. [`end-6`](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put)
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

	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let digest = query.digest;

	let header_content_length = header
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

	let is_multipart = query!(
		r#"
            SELECT 
                aws_session_id AS "aws_session_id?"
            FROM 
                container_registry_session
            WHERE 
                id = $1
            "#,
		path.session_id as _
	)
	.fetch_one(&mut *database)
	.await
	.map_err(internal_server_error_response)?
	.aws_session_id
	.is_some();

	if body.is_end_stream() {
		if !is_multipart {
			return Err(convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::BlobUploadInvalid,
				"Request body is empty".to_string(),
			));
		}

		todo!("upload body here")
	}

	query!(
		r#"
        INSERT INTO container_registry_layer_blob(
            digest,
            size
        ) VALUES (
            $1,
            $2
        );
        "#,
		&digest,
		// This is incorrect, this should come from somewhere else
		header_content_length
			.parse::<i64>()
			.map_err(internal_server_error_response)?
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	query!(
		r#"
        DELETE FROM container_registry_session
        WHERE id = $1;
        "#,
		path.session_id as _
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	Ok((
		[(
			HeaderName::from_static("Docker-Distribution-API-Version"),
			HeaderValue::from_static("registry/2.0"),
		)]
		.into_iter()
		.collect::<HeaderMap>(),
		StatusCode::OK,
	)
		.into_response())
}
