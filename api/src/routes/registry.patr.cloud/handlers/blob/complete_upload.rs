//! PUT blob upload completion endpoint handler.
//!
//! This handler completes a chunked blob upload by finalizing the S3 multipart
//! upload, verifying the digest, and storing the blob metadata in the database.

use axum::body::Body;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		AuthenticatedRegistryRequest,
		RegistryEndpoint,
		RegistryError,
		RegistryResponse,
		handlers::blob::initiate_upload::Location,
		types::RepositoryName,
		utils::{
			repository::verify_workspace_access,
			s3::{UploadPart, complete_multipart_upload, upload_part_to_s3},
		},
	},
};

/// Custom header for Docker content digest
#[derive(Debug, Clone, PartialEq)]
pub struct DockerContentDigest(String);

impl DockerContentDigest {
	pub fn new(digest: String) -> Self {
		Self(digest)
	}
}

impl headers::Header for DockerContentDigest {
	fn name() -> &'static headers::HeaderName {
		static NAME: headers::HeaderName =
			headers::HeaderName::from_static("docker-content-digest");
		&NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
	where
		I: Iterator<Item = &'i http::HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;
		let digest = value
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.to_string();
		Ok(Self(digest))
	}

	fn encode<E: Extend<http::HeaderValue>>(&self, values: &mut E) {
		if let Ok(value) = http::HeaderValue::from_str(&self.0) {
			values.extend(std::iter::once(value));
		}
	}
}

macros::declare_registry_endpoint!(
		/// PUT blob upload completion endpoint.
		///
		/// Completes a chunked blob upload by finalizing the S3 multipart upload
		/// and storing the blob metadata.
		CompleteBlobUpload,
		PUT "/v2/{name}/blobs/uploads/{uuid}" {
				/// The repository name in the format workspace_id/repo_name
				pub name: String,
				/// The upload session UUID
				pub uuid: String,
		},
		query = {
				/// The expected digest of the uploaded blob
				pub digest: String,
		},
		auth = true,
		response_headers = {
				/// Location header pointing to the blob
				pub location: Location,
				/// The digest of the uploaded blob
				pub docker_content_digest: DockerContentDigest,
		}
);

/// Handler for PUT /v2/{name}/blobs/uploads/{uuid}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Retrieves upload session from database
/// 4. Reads final chunk from request body if present
/// 5. Extracts digest from query parameter
/// 6. Completes S3 multipart upload
/// 7. Verifies digest matches uploaded content
/// 8. Stores blob metadata in database
/// 9. Deletes upload session
/// 10. Returns 201 Created with Location and Docker-Content-Digest headers
///
/// # Requirements
/// - 6.4: Upload parts to S3 incrementally
/// - 6.5: Finalize S3 multipart upload and verify digest
/// - 8.3: Verify digest matches uploaded content
/// - 8.4: Store blob metadata in database
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, CompleteBlobUploadPath>,
) -> Result<RegistryResponse<CompleteBlobUploadPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		uuid = %req.path.uuid,
		digest = %req.query.digest,
		user_id = %req.user_data.id,
		"PUT blob upload completion request"
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

	// 3. Validate digest format
	let expected_digest = &req.query.digest;
	if !expected_digest.starts_with("sha256:") || expected_digest.len() != 71 {
		warn!(
			digest = %expected_digest,
			"Invalid digest format"
		);
		return Err(RegistryError::digest_invalid(expected_digest));
	}

	// 4. Parse UUID
	let session_id = Uuid::parse_str(&req.path.uuid).map_err(|_| {
		warn!(uuid = %req.path.uuid, "Invalid upload session UUID");
		RegistryError::blob_upload_invalid(format!(
			"Invalid upload session UUID: {}",
			req.path.uuid
		))
	})?;

	// 5. Retrieve upload session from database
	#[derive(Debug)]
	struct SessionRecord {
		aws_session_id: Option<String>,
		current_part: i32,
		last_byte: i32,
		parts: Vec<SessionPart>,
	}

	#[derive(Debug, sqlx::Type)]
	#[sqlx(type_name = "container_registry_session_parts")]
	struct SessionPart {
		part_number: i32,
		etag: String,
	}

	let session = sqlx::query_as!(
		SessionRecord,
		r#"
		SELECT 
			aws_session_id,
			current_part,
			last_byte,
			parts as "parts: Vec<SessionPart>"
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
		parts_count = session.parts.len(),
		"Retrieved upload session"
	);

	// 6. Read final chunk from request body if present
	let body_stream = req.body;
	let stream_reader = StreamReader::new(
		body_stream
			.into_data_stream()
			.map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))),
	);

	let mut chunk_data = Vec::new();
	let mut reader = Box::pin(stream_reader);
	reader.read_to_end(&mut chunk_data).await.map_err(|e| {
		error!(
			session_id = %session_id,
			error = %e,
			"Failed to read final chunk from request body"
		);
		RegistryError::from(e)
	})?;

	let chunk_size = chunk_data.len();
	info!(
		session_id = %session_id,
		chunk_size = chunk_size,
		"Read final chunk from request body"
	);

	// 7. Upload final chunk if present
	let bucket = req.s3_bucket;
	let s3_key = format!("uploads/{}", session_id);
	let mut all_parts = session.parts;

	if chunk_size > 0 {
		let next_part_number = (session.current_part + 1) as u32;

		debug!(
			session_id = %session_id,
			s3_key = %s3_key,
			part_number = next_part_number,
			chunk_size = chunk_size,
			"Uploading final part to S3"
		);

		let upload_part =
			upload_part_to_s3(&bucket, &s3_key, &upload_id, next_part_number, chunk_data)
				.await
				.map_err(|e| {
					error!(
						session_id = %session_id,
						part_number = next_part_number,
						error = %e,
						"Failed to upload final part to S3"
					);
					e
				})?;

		info!(
			session_id = %session_id,
			part_number = upload_part.part_number,
			etag = %upload_part.etag,
			"Successfully uploaded final part to S3"
		);

		all_parts.push(SessionPart {
			part_number: upload_part.part_number as i32,
			etag: upload_part.etag.clone(),
		});
	}

	// 8. Complete S3 multipart upload
	let s3_parts: Vec<UploadPart> = all_parts
		.iter()
		.map(|p| UploadPart {
			part_number: p.part_number as u32,
			etag: p.etag.clone(),
		})
		.collect();

	info!(
		session_id = %session_id,
		s3_key = %s3_key,
		upload_id = %upload_id,
		parts_count = s3_parts.len(),
		"Completing S3 multipart upload"
	);

	complete_multipart_upload(&bucket, &s3_key, &upload_id, s3_parts)
		.await
		.map_err(|e| {
			error!(
				session_id = %session_id,
				error = %e,
				"Failed to complete S3 multipart upload"
			);
			e
		})?;

	info!(
		session_id = %session_id,
		"Successfully completed S3 multipart upload"
	);

	// 9. Verify digest matches uploaded content
	// Download the completed blob from S3 and compute its digest
	let blob_data = bucket.get_object(&s3_key).await.map_err(|e| {
		error!(
			session_id = %session_id,
			s3_key = %s3_key,
			error = %e,
			"Failed to retrieve completed blob from S3 for verification"
		);
		RegistryError::from_error(std::io::Error::new(
			std::io::ErrorKind::Other,
			format!("Failed to retrieve blob for verification: {}", e),
		))
	})?;

	let blob_bytes = blob_data.bytes();
	let blob_size = blob_bytes.len() as i64;

	// Compute SHA256 digest
	let mut hasher = Sha256::new();
	hasher.update(&blob_bytes);
	let computed_digest = format!("sha256:{:x}", hasher.finalize());

	info!(
		session_id = %session_id,
		computed_digest = %computed_digest,
		expected_digest = %expected_digest,
		blob_size = blob_size,
		"Computed digest for uploaded blob"
	);

	// Verify digest matches
	if computed_digest != *expected_digest {
		error!(
			session_id = %session_id,
			computed_digest = %computed_digest,
			expected_digest = %expected_digest,
			"Digest mismatch"
		);

		// Clean up the uploaded blob
		let _ = bucket.delete_object(&s3_key).await;

		return Err(RegistryError::digest_invalid(format!(
			"Digest mismatch: expected {}, got {}",
			expected_digest, computed_digest
		)));
	}

	info!(
		session_id = %session_id,
		"Digest verification successful"
	);

	// 10. Move blob from uploads/ to blobs/ in S3
	let final_s3_key = format!("blobs/{}", expected_digest);

	// Copy the blob to its final location
	bucket
		.copy_object_internal(&s3_key, &final_s3_key)
		.await
		.map_err(|e| {
			error!(
				session_id = %session_id,
				source = %s3_key,
				destination = %final_s3_key,
				error = %e,
				"Failed to move blob to final location"
			);
			RegistryError::from_error(std::io::Error::new(
				std::io::ErrorKind::Other,
				format!("Failed to move blob to final location: {}", e),
			))
		})?;

	// Delete the upload temporary file
	let _ = bucket.delete_object(&s3_key).await;

	info!(
		session_id = %session_id,
		final_key = %final_s3_key,
		"Moved blob to final location in S3"
	);

	// 11. Store blob metadata in database
	// Check if blob already exists (content-addressable storage)
	let existing_blob = sqlx::query!(
		r#"
		SELECT digest
		FROM container_registry_layer_blob
		WHERE digest = $1
		"#,
		expected_digest
	)
	.fetch_optional(&mut **req.database)
	.await?;

	if existing_blob.is_none() {
		sqlx::query!(
			r#"
			INSERT INTO container_registry_layer_blob (
				digest,
				size,
				annotations
			) VALUES ($1, $2, NULL)
			"#,
			expected_digest,
			blob_size
		)
		.execute(&mut **req.database)
		.await?;

		info!(
			digest = %expected_digest,
			size = blob_size,
			"Stored blob metadata in database"
		);
	} else {
		info!(
			digest = %expected_digest,
			"Blob already exists in database (content-addressable storage)"
		);
	}

	// 12. Delete upload session
	sqlx::query!(
		r#"
		DELETE FROM container_registry_session
		WHERE id = $1
		"#,
		session_id as _
	)
	.execute(&mut **req.database)
	.await?;

	info!(
		session_id = %session_id,
		"Deleted upload session from database"
	);

	// 13. Build Location header
	let location_url = format!("/v2/{}/blobs/{}", req.path.name, expected_digest);

	// 14. Return 201 Created with headers
	info!(
		session_id = %session_id,
		digest = %expected_digest,
		location = %location_url,
		"Blob upload completed successfully"
	);

	Ok(RegistryResponse::new(
		CompleteBlobUploadResponseHeaders {
			location: Location::new(location_url),
			docker_content_digest: DockerContentDigest::new(expected_digest.clone()),
		},
		Body::empty(),
		http::StatusCode::CREATED,
	))
}

/// Create an S3 Bucket client
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_complete_blob_upload_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<CompleteBlobUploadPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/uploads/{uuid}"
		);
	}

	#[test]
	fn test_docker_content_digest_header() {
		let digest = DockerContentDigest::new(
			"sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1".to_string(),
		);
		assert_eq!(
			digest.0,
			"sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
		);
	}
}
