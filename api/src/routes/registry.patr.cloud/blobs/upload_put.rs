use axum::{
	body::{Body, HttpBody, to_bytes},
	extract::{Path, Query, State},
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
};
use futures::TryStreamExt;
use oci_spec::distribution::ErrorCode;
use s3::serde_types::Part;
use serde::{Deserialize, Serialize};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		Error,
		get_s3_object_name_for_blob,
		get_s3_object_name_for_session,
		internal_server_error_response,
	},
	utils::helper::{
		check_repository,
		check_workspace,
		convert_oci_error,
		get_header,
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

	let repository_name = path.repo_name;
	check_repository(&repository_name, state.clone()).await?;

	let digest = query.digest;

	let header_content_length = get_header(&header, "Content-Length")?;
	let header_content_range = get_header(&header, "Content-Range")?;
	let last_byte = header_content_range
		.split('-')
		.nth(1)
		.ok_or_else(|| {
			convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::BlobUploadInvalid,
				"Invalid Content-Range header format".to_string(),
			)
		})?
		.trim()
		.parse::<u32>()
		.map_err(|_| {
			convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::BlobUploadInvalid,
				"Invalid Content-Range last byte value".to_string(),
			)
		})?;

	let mut database = state
		.database
		.begin()
		.await
		.map_err(internal_server_error_response)?;

	let bucket = get_s3_bucket(state.config.clone())?;
	let s3_key = get_s3_object_name_for_blob(&digest);

	let local_session = query!(
		r#"
            SELECT 
                aws_session_id AS "aws_session_id?",
				current_part,
				last_byte
            FROM 
                container_registry_session
            WHERE 
                id = $1
            "#,
		path.session_id as _
	)
	.fetch_one(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	let s3_session_id = local_session.aws_session_id;

	if body.is_end_stream() {
		if !s3_session_id.is_some() {
			return Err(convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::BlobUploadInvalid,
				"Request body is empty".to_string(),
			));
		}
	}

	let session_parts = query!(
		r#"
		SELECT
			(UNNEST(parts)).part_number,
			(UNNEST(parts)).etag
		FROM
			container_registry_session;
		"#
	)
	.fetch_all(&mut *database)
	.await
	.map_err(internal_server_error_response)?
	.into_iter()
	.map(|r| Part {
		// TODO: FIX THIS UNWRAP
		part_number: r.part_number.unwrap() as u32,
		etag: r.etag.unwrap(),
	})
	.collect::<Vec<_>>();

	// Upload to S3
	// Is there a better way to do this?
	if s3_session_id.is_some() {
		let s3_session_id = s3_session_id.expect("Session ID to be there");
		let s3_session_key = get_s3_object_name_for_session(path.session_id.to_string().as_str());

		if !body.is_end_stream() {
			let body_stream = to_bytes(body, usize::MAX)
				.await
				.map_err(internal_server_error_response)?;

			let mut buffer: &mut &[u8] = &mut body_stream.as_ref();

			bucket
				.put_multipart_stream(
					&mut buffer,
					s3_session_key.as_str(),
					last_byte as _,
					s3_session_id.to_string().as_str(),
					"application/octet-stream",
				)
				.await
				.map_err(internal_server_error_response)?;
		}

		bucket
			.complete_multipart_upload(
				s3_session_key.as_str(),
				// Using unwrap here cause already checking for is_some above
				s3_session_id.to_string().as_str(),
				session_parts,
			)
			.await
			.map_err(internal_server_error_response)?;

		bucket
			.copy_object_internal(s3_session_key.as_str(), &s3_key)
			.await
			.map_err(internal_server_error_response)?;
	} else {
		let mut body_stream = body
			.into_data_stream()
			.map_err(std::io::Error::other)
			.into_async_read()
			.compat();

		bucket
			.put_object_stream_with_content_type(
				&mut body_stream,
				&s3_key,
				"application/octet-stream",
			)
			.await
			.map_err(internal_server_error_response)?;

		// TODO match the content length
		let status = bucket
			.put_object_stream_with_content_type(
				&mut body_stream,
				&s3_key,
				"application/octet-stream",
			)
			.await
			.map_err(internal_server_error_response)?;

		if !(200..300).contains(&status.status_code()) {
			return Err(convert_oci_error(
				StatusCode::BAD_REQUEST,
				ErrorCode::ManifestInvalid,
				"Failed to push manifest to S3".to_string(),
			));
		}
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
		[("Docker-Distribution-API-Version", "registry/2.0")],
		StatusCode::OK,
	)
		.into_response())
}
