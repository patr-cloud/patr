use axum::{
	body::{Body, to_bytes},
	extract::{Path, State},
	http::{HeaderMap, StatusCode},
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
	reference: String,
}

/// Handles the `PATCH /v2/<name>/blobs/uploads/<reference>?digest=<digest>` route. [`end-6`](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put)
#[axum::debug_handler]
pub(super) async fn handle(
	header: HeaderMap,
	Path(path): Path<PathParams>,
	State(state): State<AppState>,
	body: Body,
) -> Result<impl IntoResponse, Error> {
	trace!("PATCH upload called on get blob");
	let path = preprocess_stuff(path)?;

	let workspace_id = path.workspace_id;
	let session_id = Uuid::parse_str(path.reference.as_str()).map_err(|_| {
		convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::BlobUploadInvalid,
			"Invalid reference, reference should be Uuid".to_string(),
		)
	})?;
	check_workspace(workspace_id, state.clone()).await?;

	let repository_name = path.repo_name;
	check_repository(&repository_name, state.clone()).await?;

	debug!("{:#?}", &header);

	// let header_content_range = get_header(&header, "Content-Range")?;
	// let last_byte = header_content_range
	// 	.split('-')
	// 	.nth(1)
	// 	.ok_or_else(|| {
	// 		convert_oci_error(
	// 			StatusCode::RANGE_NOT_SATISFIABLE,
	// 			ErrorCode::BlobUploadInvalid,
	// 			"Invalid Content-Range header format".to_string(),
	// 		)
	// 	})?
	// 	.trim()
	// 	.parse::<u32>()
	// 	.map_err(|_| {
	// 		convert_oci_error(
	// 			StatusCode::RANGE_NOT_SATISFIABLE,
	// 			ErrorCode::BlobUploadInvalid,
	// 			"Invalid Content-Range last byte value".to_string(),
	// 		)
	// 	})?;

	// trace!("chunk upload last byte: {last_byte}");

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
			current_part,
			last_byte
		FROM 
			container_registry_session
		WHERE 
			id = $1
		"#,
		session_id as _
	)
	.fetch_one(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	let s3_session_id = s3_session.aws_session_id.ok_or_else(|| {
		convert_oci_error(
			StatusCode::NOT_FOUND,
			ErrorCode::BlobUploadInvalid,
			"Invalid S3 session ID".to_string(),
		)
	})?;

	let current_part = s3_session.current_part.ok_or_else(|| {
		convert_oci_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			ErrorCode::SizeInvalid,
			"Cannot Extract Current Part".to_string(),
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
			current_part as _,
			s3_session_id.to_string().as_str(),
			"application/octet-stream",
		)
		.await
		.map_err(internal_server_error_response)?;

	trace!("uploaded body chunk");

	query!(
		r#"
		UPDATE
			container_registry_session
		SET
			parts = parts || ($1, $2)::container_registry_session_parts,
			current_part = current_part + 1,
			last_byte = $3,
			updated_at = NOW()
		WHERE
			id = $4
		"#,
		chunk_part.part_number as i32,
		chunk_part.etag,
		32i32,
		s3_session_id as _
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	database
		.commit()
		.await
		.map_err(internal_server_error_response)?;

	let location = format!(
		"https://registry.patr.cloud/v2/{}/{}/blobs/upload/{}",
		path.workspace_id, repository_name, &session_id
	);
	Ok((
		StatusCode::ACCEPTED,
		[
			("Docker-Distribution-API-Version", "registry/2.0"),
			("Location", &location),
			// ("Range", &format!("0-{}", last_byte)),
		],
	)
		.into_response())
}
