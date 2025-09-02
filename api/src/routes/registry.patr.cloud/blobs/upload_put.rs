use axum::{
	body::{Body, HttpBody, to_bytes},
	extract::{Path, Query, State},
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
};
use oci_spec::distribution::ErrorCode;
use s3::serde_types::Part;
use serde::{Deserialize, Serialize};

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
	reference: String,
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

/// Handles the `PUT /v2/<name>/blobs/uploads/<reference>?digest=<digest>` route. [`end-6`](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#post-then-put)
#[axum::debug_handler]
pub(super) async fn handle(
	header: HeaderMap,
	Path(path): Path<PathParams>,
	Query(query): Query<QueryParams>,
	State(state): State<AppState>,
	body: Body,
) -> Result<impl IntoResponse, Error> {
	trace!("PUT upload called on get blob");
	let path = preprocess_stuff(path)?;
	let query = preprocess_stuff(query)?;

	let workspace_id = path.workspace_id;
	check_workspace(workspace_id, state.clone()).await?;

	let repository_name = path.repo_name;
	check_repository(&repository_name, state.clone()).await?;

	let digest = query.digest;
	let reference = Uuid::parse_str(path.reference.as_str()).map_err(|_| {
		convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::BlobUploadInvalid,
			"Invalid reference, reference should be Uuid".to_string(),
		)
	})?;

	let header_content_length = get_header(&header, "Content-Length")?;
	// let header_content_range = get_header(&header, "Content-Range")?;
	// let last_byte = header_content_range
	// 	.split('-')
	// 	.nth(1)
	// 	.ok_or_else(|| {
	// 		convert_oci_error(
	// 			StatusCode::BAD_REQUEST,
	// 			ErrorCode::BlobUploadInvalid,
	// 			"Invalid Content-Range header format".to_string(),
	// 		)
	// 	})?
	// 	.trim()
	// 	.parse::<u32>()
	// 	.map_err(|_| {
	// 		convert_oci_error(
	// 			StatusCode::BAD_REQUEST,
	// 			ErrorCode::BlobUploadInvalid,
	// 			"Invalid Content-Range last byte value".to_string(),
	// 		)
	// 	})?;

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
		&reference as _
	)
	.fetch_one(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	let s3_session_id = local_session.aws_session_id.ok_or_else(|| {
		convert_oci_error(
			StatusCode::BAD_REQUEST,
			ErrorCode::BlobUploadInvalid,
			"Invalid S3 session ID".to_string(),
		)
	})?;

	let current_part = local_session.current_part.ok_or_else(|| {
		convert_oci_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			ErrorCode::SizeInvalid,
			"Cannot Extract Current Part".to_string(),
		)
	})?;

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

	trace!("Parts: {:#?}", session_parts);

	let s3_session_key = get_s3_object_name_for_session(&reference.to_string().as_str());

	if !body.is_end_stream() {
		let body_stream = to_bytes(body, usize::MAX)
			.await
			.map_err(internal_server_error_response)?;

		let mut buffer: &mut &[u8] = &mut body_stream.as_ref();

		bucket
			.put_multipart_stream(
				&mut buffer,
				s3_session_key.as_str(),
				current_part as _,
				s3_session_id.to_string().as_str(),
				"application/octet-stream",
			)
			.await
			.map_err(internal_server_error_response)?;
		trace!("added last body chunk");
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
	trace!("Complete Multipart Upload");

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
		&reference as _
	)
	.execute(&mut *database)
	.await
	.map_err(internal_server_error_response)?;

	bucket
		.copy_object_internal(s3_session_key.as_str(), &s3_key)
		.await
		.map_err(internal_server_error_response)?;
	trace!("moved completed object from sessions -> manifest");

	bucket
		.delete_object(s3_session_key.as_str())
		.await
		.map_err(internal_server_error_response)?;
	trace!("delete s3 sessions object");

	database
		.commit()
		.await
		.map_err(internal_server_error_response)?;

	let location = format!(
		"/v2/{}/{}/blobs/{}",
		path.workspace_id,
		&repository_name,
		digest.to_string()
	);

	Ok((
		StatusCode::CREATED,
		[
			("Docker-Distribution-API-Version", "registry/2.0"),
			("Location", &location),
		],
	)
		.into_response())
}
