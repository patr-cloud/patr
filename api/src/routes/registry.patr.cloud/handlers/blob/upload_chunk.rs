//! PATCH blob upload chunk endpoint handler.
//!
//! This handler processes chunked blob uploads, allowing clients to upload
//! large blobs in multiple parts using the OCI Distribution API's chunked
//! upload protocol.

use axum::body::Body;
use futures::StreamExt as _;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		AuthenticatedRegistryRequest,
		RegistryEndpoint,
		RegistryError,
		RegistryResponse,
		handlers::blob::initiate_upload::{DockerUploadUuid, Location, RangeHeader},
		types::RepositoryName,
		utils::{repository::verify_workspace_access, s3::upload_part_to_s3},
	},
};

macros::declare_registry_endpoint!(
	/// PATCH blob upload chunk endpoint.
	///
	/// Uploads a chunk of data to an ongoing blob upload session.
	UploadBlobChunk,
	PATCH "/v2/{name}/blobs/uploads/{uuid}" {
		/// The repository name in the format workspace_id/repo_name
		pub name: String,
		/// The upload session UUID
		pub uuid: String,
	},
	auth = true,
	response_headers = {
		/// Location header pointing to the upload URL
		pub location: Location,
		/// The UUID for this upload session
		pub docker_upload_uuid: DockerUploadUuid,
		/// The current byte range after this chunk
		pub range: RangeHeader,
	}
);

/// Handler for PATCH /v2/{name}/blobs/uploads/{uuid}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Retrieves upload session from database
/// 4. Reads chunk from streaming request body
/// 5. Uploads part to S3 using multipart upload
/// 6. Updates session in database with part number and ETag
/// 7. Updates last_byte position
/// 8. Returns 202 Accepted with Location, Range, and Docker-Upload-UUID headers
///
/// # Requirements
/// - 6.2: Upload blob chunks incrementally to S3
/// - 6.4: Upload parts to S3 incrementally
/// - 6.6: Track upload progress
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, UploadBlobChunkPath>,
) -> Result<RegistryResponse<UploadBlobChunkPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		uuid = %req.path.uuid,
		user_id = %req.user_data.id,
		"PATCH blob upload chunk request"
	);

	// 1. Parse repository name
	let repo_name = RepositoryName::parse(&req.path.name)?;
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		"Parsed repository name"
	);

	// 2. Verify workspace access
	verify_workspace_access(&req.user_data, repo_name.workspace_id())?;
	debug!("Workspace access verified");

	// 3. Parse UUID
	let session_id = Uuid::parse_str(&req.path.uuid).map_err(|_| {
		warn!(uuid = %req.path.uuid, "Invalid upload session UUID");
		RegistryError::blob_upload_invalid(format!(
			"Invalid upload session UUID: {}",
			req.path.uuid
		))
	})?;

	#[derive(Debug, sqlx::Type)]
	#[sqlx(type_name = "container_registry_session_parts")]
	struct SessionPart {
		part_number: i32,
		etag: String,
	}

	let session = sqlx::query!(
		r#"
		SELECT 
			aws_session_id,
			current_part AS "current_part!",
			last_byte as "last_byte!",
			parts AS "parts: Vec<SessionPart>"
		FROM container_registry_session
		WHERE id = $1
			AND user_id = $2
		"#,
		session_id as _,
		req.user_data.id as _
	)
	.fetch_optional(&mut **req.database)
	.await?
	.ok_or_else(|| {
		warn!(
			session_id = %session_id,
			user_id = %req.user_data.id,
			"Upload session not found"
		);
		RegistryError::blob_upload_unknown(session_id.to_string())
	})?;

	let upload_id = session.aws_session_id.ok_or_else(|| {
		error!(
			session_id = %session_id,
			"Upload session missing AWS session ID"
		);
		RegistryError::blob_upload_invalid("Upload session is not properly initialized".to_string())
	})?;

	debug!(
		session_id = %session_id,
		upload_id = %upload_id,
		current_part = session.current_part,
		last_byte = session.last_byte,
		"Retrieved upload session"
	);

	// 5. Read chunk from streaming request body
	// Convert the Body into an AsyncRead stream
	let body_stream = req.body;
	let stream_reader = StreamReader::new(
		body_stream
			.into_data_stream()
			.map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))),
	);

	// Read the chunk data
	let mut chunk_data = Vec::new();
	let mut reader = Box::pin(stream_reader);
	reader.read_to_end(&mut chunk_data).await.map_err(|e| {
		error!(
			session_id = %session_id,
			error = %e,
			"Failed to read chunk data from request body"
		);
		RegistryError::from(e)
	})?;

	let chunk_size = chunk_data.len();
	info!(
		session_id = %session_id,
		chunk_size = chunk_size,
		"Read chunk data from request body"
	);

	if chunk_size == 0 {
		warn!(
			session_id = %session_id,
			"Received empty chunk"
		);
		return Err(RegistryError::blob_upload_invalid(
			"Cannot upload empty chunk".to_string(),
		));
	}

	// 6. Upload part to S3 using multipart upload
	let bucket = req.s3_bucket;
	let s3_key = format!("uploads/{}", session_id);
	let next_part_number = (session.current_part + 1) as u32;

	debug!(
		session_id = %session_id,
		s3_key = %s3_key,
		part_number = next_part_number,
		chunk_size = chunk_size,
		"Uploading part to S3"
	);

	let upload_part = upload_part_to_s3(&bucket, &s3_key, &upload_id, next_part_number, chunk_data)
		.await
		.map_err(|e| {
			error!(
				session_id = %session_id,
				part_number = next_part_number,
				error = %e,
				"Failed to upload part to S3"
			);
			e
		})?;

	info!(
		session_id = %session_id,
		part_number = upload_part.part_number,
		etag = %upload_part.etag,
		"Successfully uploaded part to S3"
	);

	// 7. Update session in database with part number and ETag
	let new_last_byte = session.last_byte + chunk_size as i32;
	let mut updated_parts = session.parts;
	updated_parts.push(SessionPart {
		part_number: upload_part.part_number as i32,
		etag: upload_part.etag.clone(),
	});

	sqlx::query!(
		r#"
		UPDATE container_registry_session
		SET 
			current_part = $1,
			last_byte = $2,
			parts = $3,
			updated_at = NOW()
		WHERE id = $4
		"#,
		next_part_number as i32,
		new_last_byte,
		&updated_parts as &[SessionPart],
		session_id as _
	)
	.execute(&mut **req.database)
	.await?;

	info!(
		session_id = %session_id,
		current_part = next_part_number,
		last_byte = new_last_byte,
		"Updated upload session in database"
	);

	// 8. Build Location header
	let location_url = format!("/v2/{}/blobs/uploads/{}", req.path.name, session_id);

	// 9. Return 202 Accepted with headers
	info!(
		session_id = %session_id,
		location = %location_url,
		range = format!("0-{}", new_last_byte),
		"Returning upload chunk response"
	);

	Ok(RegistryResponse::new(
		UploadBlobChunkResponseHeaders {
			location: Location::new(location_url),
			docker_upload_uuid: DockerUploadUuid::new(session_id.to_string()),
			range: RangeHeader::new(0, new_last_byte as u64),
		},
		Body::empty(),
		http::StatusCode::ACCEPTED,
	))
}

/// Create an S3 Bucket client
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_upload_blob_chunk_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<UploadBlobChunkPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/uploads/{uuid}"
		);
	}
}
