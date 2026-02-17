//! PATCH blob upload chunk endpoint handler.
//!
//! This handler processes chunked blob uploads, allowing clients to upload
//! large blobs in multiple parts using the OCI Distribution API's chunked
//! upload protocol.

use std::{self, str::FromStr};

use aws_sdk_s3::{operation::upload_part::UploadPartOutput, primitives::ByteStream};
use axum::body::Body;
use base64::prelude::*;
use futures::StreamExt;
use headers::{ContentLength, ContentRange, ContentType};
use http_body::Body as _;
use models::utils::{DockerUploadUuid, Range};
use rustis::commands::{GenericCommands, StringCommands};
use tokio::task::JoinHandle;

use crate::{redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// PATCH blob upload chunk endpoint.
	///
	/// Uploads a chunk of data to an ongoing blob upload session.
	UploadBlobChunk,
	PATCH "/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The upload session UUID
		pub session_id: Uuid,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
		/// The content type header
		pub content_type: ContentType,
		/// The content length header
		pub content_length: OptionalHeader<ContentLength>,
		/// The content range header
		pub content_range: OptionalHeader<ContentRange>,
	},
	response_headers = {
		/// Location header pointing to the upload URL
		pub location: Location,
		/// The UUID for this upload session
		pub docker_upload_uuid: DockerUploadUuid,
		/// The current byte range after this chunk
		pub range: Range,
	}
);

/// Handler for PATCH /v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}
///
/// This handler:
/// - Verifies workspace access
/// - Retrieves upload session from redis
/// - Reads chunk from streaming request body
/// - Uploads part to S3 using multipart upload
/// - Updates session in redis with part number and ETag
/// - Updates last_byte position
/// - Returns 202 Accepted with Location, Range, and Docker-Upload-UUID headers
pub async fn upload_chunk(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					UploadBlobChunkPathProcessed {
						workspace_id,
						repo_name,
						session_id,
					},
				query: (),
				headers:
					UploadBlobChunkRequestHeaders {
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
	}: AuthenticatedRegistryAppRequest<'_, UploadBlobChunkPath>,
) -> Result<RegistryResponse<UploadBlobChunkPath>, RegistryError> {
	info!("PATCH blob upload chunk request");
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

	let session = serde_json::from_str::<S3UploadSession>(
		&redis
			.get::<String>(keys::registry_blob_upload_part_prefix(&session_id))
			.await?,
	)?;
	trace!("Retrieved upload session");

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

	if content_type != ContentType::octet_stream() {
		warn!(
			"Invalid Content-Type for blob chunk upload: {}",
			content_type
		);
		return RegistryError::builder()
			.code(ErrorCode::BlobUploadInvalid)
			.message("Content-Type must be application/octet-stream for blob chunk upload")
			.status(StatusCode::BAD_REQUEST)
			.build()
			.into_result();
	}

	if body.is_end_stream() {
		warn!("Empty body provided for blob chunk upload");
		return RegistryError::builder()
			.code(ErrorCode::BlobUploadInvalid)
			.message("Body must not be empty for blob chunk upload")
			.status(StatusCode::BAD_REQUEST)
			.build()
			.into_result();
	}

	if let Some(content_range) = content_range.into_option() {
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
	}

	// Upload chunk to S3

	let mut uploaded_parts_etags = session.uploaded_parts_etags;
	let mut total_bytes_uploaded = session.total_bytes_uploaded;

	if let Some(ContentLength(content_length)) = content_length.into_option() &&
		content_length > 0
	{
		// Content-Length is known — stream the body directly as a single part
		let part_number = uploaded_parts_etags.len() as i32 + 1;
		info!("Uploading part {part_number} to S3 (streamed, {content_length}B)");

		let response = s3
			.upload_part()
			.bucket(&config.s3.bucket)
			.key(format!("uploads/{}", session_id))
			.content_length(content_length as i64)
			.upload_id(&session.upload_id)
			.part_number(part_number)
			.body(BodyStreamWrapper::new(body.into_data_stream()).into_byte_stream())
			.send()
			.await?;

		uploaded_parts_etags.push(response.e_tag.ok_or_else(|| {
			RegistryError::builder()
				.code(ErrorCode::BlobUploadInvalid)
				.message("Missing or invalid ETag header in S3 response")
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.build()
		})?);
		total_bytes_uploaded += content_length as u64;

		info!("Uploaded part {part_number} to S3 successfully");
	} else {
		// No Content-Length — buffer bytes and flush to S3 every 5 MB
		info!("Uploading chunk in buffered mode (no Content-Length)");

		/// 5 MB threshold for buffered chunked uploads — S3 requires all
		/// non-final parts to be at least this size.
		const CHUNK_FLUSH_THRESHOLD: usize = 5 * 1024 * 1024;

		// Load any pending buffer from a previous PATCH that was under 5 MB
		let pending_key = keys::registry_blob_upload_pending_buffer(&session_id);
		let mut buffer = match redis.get::<Option<String>>(&pending_key).await? {
			Some(encoded) => {
				info!("Loaded pending buffer from Redis");
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

		let mut stream = body.into_data_stream();
		let mut inflight_task = None::<JoinHandle<Result<(i64, UploadPartOutput), _>>>;

		// Read chunks from the body stream, flushing to S3 whenever the buffer
		// exceeds the 5 MB threshold.
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
				let (chunk_len, response) = task.await.expect("push task panicked")?;

				uploaded_parts_etags.push(response.e_tag.ok_or_else(|| {
					RegistryError::builder()
						.code(ErrorCode::BlobUploadInvalid)
						.message("Missing or invalid ETag header in S3 response")
						.status(StatusCode::INTERNAL_SERVER_ERROR)
						.build()
				})?);

				total_bytes_uploaded += chunk_len as u64;
			}

			// Spawn the upload for the current buffer and start a fresh one.
			let part_number = uploaded_parts_etags.len() as i32 + 1;
			info!(
				"Buffer reached {}B, flushing as part {part_number}",
				buffer.len()
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

			buffer = Vec::with_capacity(CHUNK_FLUSH_THRESHOLD);
		}

		// Stream ended — collect the last in-flight upload, if any.
		if let Some(task) = inflight_task.take() {
			let (chunk_len, response) = task.await.expect("push task panicked")?;

			uploaded_parts_etags.push(response.e_tag.ok_or_else(|| {
				RegistryError::builder()
					.code(ErrorCode::BlobUploadInvalid)
					.message("Missing or invalid ETag header in S3 response")
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.build()
			})?);

			total_bytes_uploaded += chunk_len as u64;
		}

		// Any leftover bytes that didn't reach the flush threshold are stored
		// in Redis as a pending buffer — uploading them as a sub-5 MB S3 part
		// would violate the minimum part size requirement for non-final parts.
		if !buffer.is_empty() {
			info!("Storing {}B pending buffer in Redis", buffer.len());
			redis
				.setex(
					&pending_key,
					constants::REGISTRY_BLOB_UPLOAD_PENDING_BUFFER_TTL.as_secs(),
					BASE64_STANDARD.encode(&buffer),
				)
				.await?;
			total_bytes_uploaded += buffer.len() as u64;
		} else {
			// No leftover bytes — clean up any previous pending buffer
			let _ = redis.del(&pending_key).await;
		}
	}

	// Persist updated session & respond

	info!("All parts uploaded successfully");

	let updated_session = S3UploadSession {
		upload_id: session.upload_id,
		uploaded_parts_etags,
		total_bytes_uploaded,
		initiated_by_login: session.initiated_by_login,
		initiated_by_ip: session.initiated_by_ip,
	};

	redis
		.setex(
			keys::registry_blob_upload_part_prefix(&session_id),
			constants::REGISTRY_BLOB_UPLOAD_SESSION_TTL.as_secs(),
			serde_json::to_string(&updated_session)?,
		)
		.await?;

	RegistryResponse::builder()
		.status_code(StatusCode::ACCEPTED)
		.headers(UploadBlobChunkResponseHeaders {
			location: Location::from_str(&format!(
				"/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}"
			))?,
			docker_upload_uuid: DockerUploadUuid::new(session_id),
			range: Range::new(0..updated_session.total_bytes_uploaded).map_err(|err| {
				error!("Invalid range error: {}", err);
				RegistryError::builder()
					.code(ErrorCode::SizeInvalid)
					.message(
						if cfg!(debug_assertions) {
							format!("invalid range specified: {}", err)
						} else {
							"invalid range specified".to_string()
						},
					)
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.build()
			})?,
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
	fn test_upload_blob_chunk_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<UploadBlobChunkPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/uploads/{uuid}"
		);
	}
}
