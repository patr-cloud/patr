use axum::{
	body::{Body, to_bytes},
	extract::{Path, State},
	http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
	response::IntoResponse,
};
use oci_spec::distribution::ErrorCode;
use serde::{Deserialize, Serialize};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		Error,
		get_s3_object_name_for_session,
		internal_server_error_response,
	},
	utils::helper::{
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

/// Handles the `GET /v2/<name>/blobs/uploads/<reference>?digest=<digest>` route. [`end-6`](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put)
#[axum::debug_handler]
pub(super) async fn handle(
	header: HeaderMap,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
	body: Body,
) -> Result<impl IntoResponse, Error> {
	let path = preprocess_stuff(path)?;

	let workspace_id = path.workspace_id;
	let session_id = path.session_id;
	check_workspace(workspace_id, state.clone()).await?;

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

	let s3_session = query!(
		r#"
		SELECT 
			aws_session_id AS "aws_session_id?",
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

	let s3_session_id = s3_session.aws_session_id.ok_or_else(|| {
		convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::BlobUploadInvalid,
			"Invalid S3 session ID".to_string(),
		)
	})?;

	// dunno if this is right
	// if s3_session
	// 	.last_byte
	// 	.is_some_and(|x| (x + 1) as u32 != last_byte)
	// {
	// 	return Err(convert_oci_error(
	// 		StatusCode::BAD_REQUEST,
	// 		ErrorCode::BlobUploadInvalid,
	// 		"Invalid Content-Range last byte value".to_string(),
	// 	));
	// }

	let s3_key = get_s3_object_name_for_session(session_id.to_string().as_str());
	let body_stream = to_bytes(body, usize::MAX)
		.await
		.map_err(internal_server_error_response)?;

	let mut buffer: &mut &[u8] = &mut body_stream.as_ref();
	let chunk_part = bucket
		.put_multipart_stream(
			&mut buffer,
			s3_key.as_str(),
			last_byte as _,
			s3_session_id.to_string().as_str(),
			"application/octet-stream",
		)
		.await
		.map_err(internal_server_error_response)?;

	query!(
		r#"
		UPDATE
			container_registry_session
		SET
			parts = parts || ($1, $2)::container_registry_session_parts,
			current_part = current_part + 1,
			last_byte = $3
		WHERE
			id = $4
		"#,
		chunk_part.part_number as i32,
		chunk_part.etag,
		last_byte as i32,
		s3_session_id as _
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
