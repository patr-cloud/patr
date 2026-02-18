//! PUT blob upload completion endpoint handler.
//!
//! This handler completes a chunked blob upload by finalizing the S3 multipart
//! upload, verifying the digest, and storing the blob metadata in the database.

use std::str::FromStr;

use aws_sdk_s3::{
	operation::upload_part::UploadPartOutput,
	primitives::ByteStream,
	types::{CompletedMultipartUpload, CompletedPart},
};
use axum::body::Body;
use base64::prelude::*;
use futures::StreamExt;
use headers::{ContentLength, ContentRange, ContentType};
use http_body::Body as _;
use rustis::commands::{GenericCommands, StringCommands};
use sha2::{Digest, Sha256};
use tokio::task::JoinHandle;

use crate::{redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// PUT blob upload completion endpoint.
	///
	/// Completes a chunked blob upload by finalizing the S3 multipart upload
	/// and storing the blob metadata.
	CompleteBlobUpload,
	PUT "/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The upload session UUID
		pub session_id: Uuid,
	},
	query = {
		/// The expected digest of the uploaded blob
		pub digest: String,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
		/// The content type of the request body
		pub content_type: OptionalHeader<ContentType>,
		/// The content length of the request body
		pub content_length: OptionalHeader<ContentLength>,
		/// The content range of the request body
		pub content_range: OptionalHeader<ContentRange>,
	},
	response_headers = {
		/// Location header pointing to the blob
		pub location: Location,
		/// The digest of the uploaded blob
		pub docker_content_digest: DockerContentDigest,
	}
);

/// Handler for PUT /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}
///
/// This handler:
/// - Verifies workspace access
/// - Retrieves upload session from redis
/// - Reads final chunk from request body if present
/// - Extracts digest from query parameter
/// - Completes S3 multipart upload
/// - Verifies digest matches uploaded content
/// - Stores blob metadata in database
/// - Deletes upload session
/// - Returns 201 Created with Location and Docker-Content-Digest headers
pub async fn complete_upload(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					CompleteBlobUploadPathProcessed {
						workspace_id,
						repo_name,
						session_id,
					},
				query: CompleteBlobUploadQueryProcessed { digest },
				headers:
					CompleteBlobUploadRequestHeaders {
						authorization: _,
						content_type,
						content_length,
						content_range,
					},
				body,
			},
		database,
		redis,
		s3,
		client_ip,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, CompleteBlobUploadPath>,
) -> Result<RegistryResponse<CompleteBlobUploadPath>, RegistryError> {
	info!("PUT blob upload completion request");

	// Check that the user can push to this repository
	let (repository_id, permission_id) = query!(
		r#"
		SELECT
			id AS "resource_id: Uuid",
			(
				SELECT
					id
				FROM
					permission
				WHERE
					name = $3
			) AS "permission_id!: Uuid"
		FROM
			container_registry_repository
		WHERE
			workspace_id = $1 AND
			name = $2 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
		&repo_name,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push)
			.to_string(),
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or_else(|| {
		warn!("Repository `{workspace_id}/{repo_name}` not found");
		RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Repository not found")
			.code(ErrorCode::NameUnknown)
			.build()
	})
	.map(|row| (row.resource_id, row.permission_id))?;

	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, permission_id);

	if !authorized {
		// Intentionally return a 404 to avoid leaking repository existence
		debug!("User not authorized to access repository");
		return RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Repository not found")
			.code(ErrorCode::NameUnknown)
			.build()
			.into_result();
	}

	let mut session = serde_json::from_str::<S3UploadSession>(
		&redis
			.get::<String>(keys::registry_blob_upload_part_prefix(&session_id))
			.await?,
	)?;

	if user_data.login_id != session.initiated_by_login {
		warn!(
			"User login `{}` does not match upload session initiator `{}`",
			user_data.login_id, session.initiated_by_login
		);
		return RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Repository not found")
			.code(ErrorCode::NameUnknown)
			.build()
			.into_result();
	}

	if client_ip != session.initiated_by_ip {
		warn!(
			"Client IP `{}` does not match upload session initiator IP `{}`",
			client_ip, session.initiated_by_ip
		);
		return RegistryError::builder()
			.status(StatusCode::UNAUTHORIZED)
			.message("Your IP address has changed since the upload was initiated")
			.code(ErrorCode::Unauthorized)
			.build()
			.into_result();
	}

	debug!("Retrieved upload session");

	// Stream the final chunk body (if present) combined with any pending
	// buffer from Redis, flushing to S3 in uniform-sized parts. Only the
	// very last part may be smaller than the threshold, satisfying R2's
	// "all non-trailing parts must have equal length" constraint.

	/// 5 MB threshold — must match the flush size used in upload_chunk so
	/// that all non-trailing parts across the entire upload are equal.
	const CHUNK_FLUSH_THRESHOLD: usize = 5 * 1024 * 1024;

	// Load any pending buffer from Redis (leftover from previous PATCH
	// requests that was under the 5MB minimum part size).
	let pending_key = keys::registry_blob_upload_pending_buffer(&session_id);
	let mut buffer = match redis.get::<Option<String>>(&pending_key).await? {
		Some(encoded) => {
			let _ = redis.del(&pending_key).await;
			BASE64_STANDARD.decode(&encoded).map_err(|err| {
				error!("Failed to decode pending buffer from Redis: {err}");
				RegistryError::builder()
					.code(ErrorCode::BlobUploadInvalid)
					.message("Corrupted pending upload buffer")
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.build()
			})?
		}
		None => Vec::with_capacity(CHUNK_FLUSH_THRESHOLD),
	};

	// Stream body chunks (if any) into the buffer, flushing full 5MB parts.
	'upload: {
		let params = content_type
			.into_option()
			.zip(content_length.into_option())
			.zip(content_range.into_option());

		let Some(((content_type, content_length), content_range)) = params else {
			break 'upload;
		};

		if content_length.0 == 0 {
			break 'upload;
		}

		if body.is_end_stream() {
			warn!("Empty body provided for blob upload");
			return RegistryError::builder()
				.code(ErrorCode::BlobUploadInvalid)
				.message("Body must not be empty for blob upload")
				.status(StatusCode::BAD_REQUEST)
				.build()
				.into_result();
		}

		trace!(
			"Streaming final chunk: content_length={}, content_range={:?}, content_type={}",
			content_length.0, content_range, content_type
		);

		if content_type != ContentType::octet_stream() {
			warn!(
				"Invalid Content-Type for single blob upload: {}",
				content_type
			);
			return RegistryError::builder()
				.code(ErrorCode::BlobUploadInvalid)
				.message("Content-Type must be application/octet-stream for single blob upload")
				.status(StatusCode::BAD_REQUEST)
				.build()
				.into_result();
		}

		let start_range = content_range
			.bytes_range()
			.map(|range| range.0)
			.unwrap_or_default();
		if start_range != session.total_bytes_uploaded {
			warn!(
				"Content-Range start is {start_range} but last byte position is {}",
				session.total_bytes_uploaded
			);
			return RegistryError::builder()
				.code(ErrorCode::BlobUploadInvalid)
				.message("Content-Range start does not match expected byte position")
				.status(StatusCode::RANGE_NOT_SATISFIABLE)
				.build()
				.into_result();
		}

		let mut stream = body.into_data_stream();
		let mut inflight_task = None::<JoinHandle<Result<(i64, UploadPartOutput), _>>>;

		while let Some(chunk) = stream.next().await {
			let data = chunk.map_err(|err| {
				error!("Failed to read body frame: {err}");
				RegistryError::builder()
					.code(ErrorCode::BlobUploadInvalid)
					.message("Failed to read request body")
					.status(StatusCode::BAD_REQUEST)
					.build()
			})?;

			buffer.extend(data);

			if buffer.len() <= CHUNK_FLUSH_THRESHOLD {
				continue;
			}

			// Buffer exceeds threshold — wait for any in-flight upload to
			// finish before starting a new one (preserves part ordering).
			if let Some(task) = inflight_task.take() {
				let (_, response) = task.await.expect("upload task panicked")?;

				session
					.uploaded_parts_etags
					.push(response.e_tag.ok_or_else(|| {
						RegistryError::builder()
							.code(ErrorCode::BlobUploadInvalid)
							.message("Missing or invalid ETag header in S3 response")
							.status(StatusCode::INTERNAL_SERVER_ERROR)
							.build()
					})?);
			}

			// Split the buffer at exactly the threshold so every
			// non-trailing part is exactly CHUNK_FLUSH_THRESHOLD bytes.
			let remainder = buffer.split_off(CHUNK_FLUSH_THRESHOLD + 1);

			let part_number = session.uploaded_parts_etags.len() as i32 + 1;
			info!(
				"Flushing exactly {:.2}MB as part {part_number}",
				buffer.len() as f64 / (1024.0 * 1024.0)
			);

			let s3 = s3.clone();
			let bucket = config.s3.bucket.clone();
			let upload_id = session.upload_id.clone();

			inflight_task = Some(tokio::spawn(async move {
				let chunk_len = buffer.len() as i64;
				s3.upload_part()
					.bucket(&bucket)
					.key(format!("uploads/{}", session_id))
					.upload_id(&upload_id)
					.part_number(part_number)
					.content_length(chunk_len)
					.body(ByteStream::from(buffer))
					.send()
					.await
					.map(|response| (chunk_len, response))
					.map_err(|err| {
						error!("Failed to upload part to S3: {err}");
						RegistryError::builder()
							.code(ErrorCode::BlobUploadInvalid)
							.message("Failed to upload chunk to storage")
							.status(StatusCode::INTERNAL_SERVER_ERROR)
							.build()
					})
			}));

			buffer = remainder;
		}

		// Stream ended — collect the last in-flight upload, if any.
		if let Some(task) = inflight_task.take() {
			let (_, response) = task.await.expect("upload task panicked")?;

			session
				.uploaded_parts_etags
				.push(response.e_tag.ok_or_else(|| {
					RegistryError::builder()
						.code(ErrorCode::BlobUploadInvalid)
						.message("Missing or invalid ETag header in S3 response")
						.status(StatusCode::INTERNAL_SERVER_ERROR)
						.build()
				})?);
		}
	}

	// Upload whatever remains in the buffer as the final trailing part.
	// This is the only part allowed to be smaller than CHUNK_FLUSH_THRESHOLD.
	if !buffer.is_empty() {
		let part_number = session.uploaded_parts_etags.len() as i32 + 1;
		let bytes_to_upload = buffer.len() as i64;
		info!(
			"Uploading {:.2}MB as final trailing part {part_number}",
			bytes_to_upload as f64 / (1024.0 * 1024.0)
		);

		let response = s3
			.upload_part()
			.bucket(&config.s3.bucket)
			.key(format!("uploads/{}", session_id))
			.upload_id(&session.upload_id)
			.part_number(part_number)
			.content_length(bytes_to_upload)
			.body(ByteStream::from(buffer))
			.send()
			.await?;

		session
			.uploaded_parts_etags
			.push(response.e_tag.ok_or_else(|| {
				RegistryError::builder()
					.code(ErrorCode::BlobUploadInvalid)
					.message("Missing or invalid ETag header in S3 response")
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.build()
			})?);
	}

	info!("Completing S3 multipart upload");
	s3.complete_multipart_upload()
		.bucket(&config.s3.bucket)
		.key(format!("uploads/{session_id}"))
		.upload_id(&session.upload_id)
		.multipart_upload(
			CompletedMultipartUpload::builder()
				.set_parts(Some(
					session
						.uploaded_parts_etags
						.into_iter()
						.enumerate()
						.map(|(index, etag)| {
							CompletedPart::builder()
								.part_number(index as i32 + 1)
								.e_tag(etag)
								.build()
						})
						.collect(),
				))
				.build(),
		)
		.send()
		.await?;
	info!("Successfully completed S3 multipart upload");

	let mut object = s3
		.get_object()
		.bucket(&config.s3.bucket)
		.key(format!("uploads/{session_id}"))
		.send()
		.await
		.inspect_err(|e| {
			error!("Failed to head blob object in S3: {e}");
		})?;

	trace!("Updating the database and completing the upload");
	query!(
		r#"
		INSERT INTO
			container_registry_blob(
				digest,
				size
			)
		VALUES
			($1, $2)
		ON CONFLICT (digest) DO NOTHING;
		"#,
		&digest,
		object.content_length().unwrap_or_default()
	)
	.execute(&mut **database)
	.await?;

	// Temporarily associate this blob with the repository in Redis so that
	// HEAD/GET blob checks pass before the manifest is pushed.
	redis
		.setex(
			keys::repository_for_registry_blob(&repository_id, &digest),
			constants::REGISTRY_BLOB_UPLOAD_SESSION_TTL.as_secs(),
			"exists",
		)
		.await?;

	// Verify digest matches uploaded content
	// Download the completed blob from S3 and compute its digest
	let mut hasher = Sha256::new();
	while let Some(bytes) = object.body.try_next().await? {
		hasher.update(&bytes);
	}
	let computed_digest = format!("sha256:{:x}", hasher.finalize());

	info!("Computed digest for uploaded blob");

	// Verify digest matches
	if computed_digest != digest {
		error!("Digest mismatch");

		// Clean up the uploaded blob
		let _ = s3
			.delete_object()
			.bucket(&config.s3.bucket)
			.key(format!("uploads/{session_id}"))
			.send()
			.await;

		return RegistryError::builder()
			.status(StatusCode::BAD_REQUEST)
			.code(ErrorCode::DigestInvalid)
			.message(format!(
				"Digest mismatch: expected {digest}, got {computed_digest}",
			))
			.build()
			.into_result();
	}

	info!("Digest verification successful");

	s3.copy_object()
		.bucket(&config.s3.bucket)
		.copy_source(format!("{}/uploads/{session_id}", config.s3.bucket))
		.key(format!("blobs/{}", digest))
		.send()
		.await?;

	let _ = s3
		.delete_object()
		.bucket(&config.s3.bucket)
		.key(format!("uploads/{session_id}"))
		.send()
		.await;

	info!("Moved blob to final location in S3");

	// Delete upload session
	redis
		.del(keys::registry_blob_upload_part_prefix(&session_id))
		.await?;

	info!("Deleted upload session from redis");

	RegistryResponse::builder()
		.status_code(StatusCode::CREATED)
		.headers(CompleteBlobUploadResponseHeaders {
			location: Location::from_str(&format!(
				"/v2/{workspace_id}/{repo_name}/blobs/{digest}",
			))?,
			docker_content_digest: DockerContentDigest(digest),
		})
		.body(Body::empty())
		.build()
		.into_result()
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
			"/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}"
		);
	}

	#[test]
	fn test_docker_content_digest_header() {
		let digest = DockerContentDigest(
			"sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1".to_string(),
		);
		assert_eq!(
			digest.0,
			"sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
		);
	}
}
