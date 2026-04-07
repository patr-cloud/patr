//! PUT blob upload completion endpoint handler.
//!
//! This handler completes a chunked blob upload by finalizing the S3 multipart
//! upload, verifying the digest, and storing the blob metadata in the database.

use std::str::FromStr;

use aws_sdk_s3::{
	error::SdkError,
	operation::upload_part::{UploadPartError, UploadPartOutput},
	primitives::ByteStream,
	types::{CompletedMultipartUpload, CompletedPart},
};
use axum::body::{Body, Bytes};
use base64::prelude::*;
use futures::{StreamExt, TryStreamExt as _};
use headers::{ContentLength, ContentRange, ContentType};
use oci_spec::image::Digest as OciDigest;
use rustis::commands::{GenericCommands, StringCommands};
use sha2::{
	Digest,
	Sha256,
	digest::common::hazmat::{SerializableState, SerializedState},
};
use tokio::task::JoinHandle;

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

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
						content_type: _,
						content_length: _,
						content_range: _,
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
	let repository_id = query!(
		r#"
		SELECT
			id AS "resource_id: Uuid"
		FROM
			container_registry_repository
		WHERE
			workspace_id = $1 AND
			name = $2 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
		&repo_name,
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
	.map(|row| row.resource_id)?;

	let permission_id = permissions::get_permission_id(
		database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push),
	)
	.await;

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

	let session = serde_json::from_str::<S3UploadSession>(
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

	let hasher = Sha256::deserialize(&SerializedState::<Sha256>::from_iter(
		hex::decode(&session.hasher_state).map_err(|err| {
			error!("Failed to decode hasher state from session: {err}");
			RegistryError::server_error(
				ErrorCode::BlobUploadInvalid,
				"Corrupted upload session hasher state",
			)
		})?,
	))
	.map_err(|err| {
		error!("Failed to deserialize hasher state: {err}");
		RegistryError::server_error(
			ErrorCode::BlobUploadInvalid,
			"Corrupted upload session hasher state",
		)
	})?;

	// Stream the final chunk body (if present) combined with any pending
	// buffer from Redis, flushing to S3 in uniform-sized parts. Only the
	// very last part may be smaller than the threshold, satisfying R2's
	// "all non-trailing parts must have equal length" constraint.

	/// 5 MB threshold — must match the flush size used in upload_chunk so
	/// that all non-trailing parts across the entire upload are equal.
	const CHUNK_FLUSH_THRESHOLD: u64 = 5 * 1024 * 1024;

	// Load any pending buffer from Redis (leftover from previous PATCH
	// requests that was under the 5MB minimum part size).
	let pending_key = keys::registry_blob_upload_pending_buffer(&session_id);
	let buffer = redis
		.get::<Option<String>>(&pending_key)
		.await?
		.map(|encoded| {
			// If there's any pending buffer, decode it from base64 and prepend it to the
			// body stream
			info!("Loaded pending buffer from Redis");
			BASE64_STANDARD.decode(&encoded).map_err(|err| {
				error!("Failed to decode pending buffer from Redis: {err}");
				RegistryError::server_error(
					ErrorCode::BlobUploadInvalid,
					"Corrupted pending upload buffer",
				)
			})
		})
		.transpose()?
		.unwrap_or_default();

	let (updated_session, hasher, _) =
		futures::stream::once(async move { Ok(Bytes::from(buffer)) })
			.chain(body.into_data_stream().map_err(|err| {
				error!("Failed to read body stream: {err}");
				RegistryError::builder()
					.code(ErrorCode::BlobUploadInvalid)
					.message("Failed to read request body")
					.status(StatusCode::BAD_REQUEST)
					.build()
			}))
			.read_buffered_bytes(CHUNK_FLUSH_THRESHOLD)
			.chain(futures::stream::once(async { Ok(Bytes::new()) }))
			.try_fold(
				(
					session,
					hasher,
					None::<JoinHandle<Result<UploadPartOutput, SdkError<UploadPartError>>>>,
				),
				async |(mut session, mut hasher, inflight_task), chunk| {
					let part_number = session.uploaded_parts_etags.len() as i32 + 1;
					let chunk_size = chunk.len();
					let chunk_size_string =
						format!("{:.2}MB", chunk_size as f64 / (1024.0 * 1024.0));

					info!(
						"Uploading part {part_number} to S3 (buffered, {})",
						chunk_size_string
					);

					if let Some(task) = inflight_task {
						let response = task.await.expect("push task panicked")?;

						session
							.uploaded_parts_etags
							.push(response.e_tag.ok_or_else(|| {
								RegistryError::server_error(
									ErrorCode::BlobUploadInvalid,
									"Missing or invalid ETag header in S3 response",
								)
							})?);

						session.total_bytes_uploaded += CHUNK_FLUSH_THRESHOLD;
					}

					let inflight_task = match chunk_size as u64 {
						CHUNK_FLUSH_THRESHOLD => {
							hasher.update(&chunk);

							Some(tokio::spawn({
								let s3 = s3.clone();
								let bucket = config.s3.bucket.clone();
								let upload_id = session.upload_id.clone();
								async move {
									s3.upload_part()
										.bucket(&bucket)
										.key(format!("registry/uploads/{}", session_id))
										.content_length(CHUNK_FLUSH_THRESHOLD as i64)
										.upload_id(&upload_id)
										.part_number(part_number)
										.body(ByteStream::from(chunk))
										.send()
										.await
								}
							}))
						}
						0 => None,
						..CHUNK_FLUSH_THRESHOLD => {
							// Upload whatever remains in the buffer as the final trailing part.
							// This is the only part allowed to be smaller than
							// CHUNK_FLUSH_THRESHOLD.
							info!(
								"Uploading {:.2}MB as final trailing part {part_number}",
								chunk_size_string
							);

							hasher.update(&chunk);

							let response = s3
								.upload_part()
								.bucket(&config.s3.bucket)
								.key(format!("registry/uploads/{}", session_id))
								.upload_id(&session.upload_id)
								.part_number(part_number)
								.content_length(chunk_size as i64)
								.body(ByteStream::from(chunk))
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
							session.total_bytes_uploaded += chunk_size as u64;
							None
						}
						CHUNK_FLUSH_THRESHOLD.. => {
							error!(
								"Chunk size {} exceeds flush threshold of {}",
								chunk_size, CHUNK_FLUSH_THRESHOLD
							);
							return Err(RegistryError::server_error(
								ErrorCode::BlobUploadInvalid,
								"Chunk size exceeds flush threshold",
							));
						}
					};

					Ok((session, hasher, inflight_task))
				},
			)
			.await?;

	info!("Completing S3 multipart upload");
	s3.complete_multipart_upload()
		.bucket(&config.s3.bucket)
		.key(format!("registry/uploads/{session_id}"))
		.upload_id(&updated_session.upload_id)
		.multipart_upload(
			CompletedMultipartUpload::builder()
				.set_parts(Some(
					updated_session
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

	// Clean up the pending buffer in Redis
	let _ = redis.del(&pending_key).await;

	trace!("Updating the database and completing the upload");
	let inserted = query!(
		r#"
		INSERT INTO
			container_registry_blob(
				digest,
				size
			)
		VALUES
			($1, $2)
		ON CONFLICT (digest) DO NOTHING
		RETURNING digest;
		"#,
		&digest,
		updated_session.total_bytes_uploaded as i64
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	// Temporarily associate this blob with the repository in Redis so that
	// HEAD/GET blob checks pass before the manifest is pushed.
	redis
		.setex(
			keys::repository_for_registry_blob(&repository_id, &digest),
			constants::REGISTRY_BLOB_UPLOAD_SESSION_TTL.as_secs(),
			"exists",
		)
		.await?;

	let computed_digest = hex::encode(hasher.finalize());
	let reference_digest = OciDigest::from_str(&digest).ok();
	let digest_mismatch = reference_digest
		.as_ref()
		.map(|digest| digest.digest() != computed_digest)
		.unwrap_or(false);

	info!("Computed digest for uploaded blob");

	// Verify digest matches
	if digest_mismatch {
		error!("Digest mismatch");

		// Clean up the uploaded blob
		let _ = s3
			.delete_object()
			.bucket(&config.s3.bucket)
			.key(format!("registry/uploads/{session_id}"))
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

	if inserted {
		s3.copy_object()
			.bucket(&config.s3.bucket)
			.copy_source(format!(
				"{}/registry/uploads/{session_id}",
				config.s3.bucket
			))
			.key(format!("registry/blobs/{}", digest))
			.send()
			.await?;
	}

	let _ = s3
		.delete_object()
		.bucket(&config.s3.bucket)
		.key(format!("registry/uploads/{session_id}"))
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
