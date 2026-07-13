//! PATCH blob upload chunk endpoint handler.
//!
//! This handler processes chunked blob uploads, allowing clients to upload
//! large blobs in multiple parts using the OCI Distribution API's chunked
//! upload protocol.

use std::{self, str::FromStr};

use aws_sdk_s3::{
	error::SdkError,
	operation::upload_part::{UploadPartError, UploadPartOutput},
	primitives::ByteStream,
};
use axum::body::{Body, Bytes};
use base64::prelude::*;
use futures::{StreamExt, TryStreamExt};
use headers::{ContentLength, ContentType};
use http_body::Body as _;
use models::utils::{ContentRange, DockerUploadUuid, Range};
use rustis::commands::{GenericCommands, StringCommands};
use sha2::{
	Digest as _,
	Sha256,
	digest::common::hazmat::{SerializableState, SerializedState},
};
use tokio::task::JoinHandle;

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

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
		/// The content type header. Optional: Docker 24 and earlier send a
		/// chunked PATCH (Transfer-Encoding: chunked) with no Content-Type, which
		/// is spec-compliant — a missing value is treated as octet-stream below.
		pub content_type: OptionalHeader<ContentType>,
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
						content_length: _,
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

	// Only validate when a Content-Type is actually present; an absent one (as
	// Docker 24's chunked PATCH sends) defaults to application/octet-stream.
	if let Some(content_type) = content_type.into_option() {
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

	// Load any pending buffer from a previous PATCH that was under 5 MB. We
	// need its size before the Content-Range check below so that the byte
	// position we compare against reflects bytes received (S3 + Redis), not
	// just bytes flushed to S3.
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

	let pending_size_before = buffer.len() as u64;

	if let Some(content_range) = content_range.into_option() {
		let start_range = content_range.start().unwrap_or_default();
		let received_so_far = session.total_bytes_uploaded + pending_size_before;
		if start_range != received_so_far {
			warn!(
				"Content-Range start is {start_range} but last byte position is {received_so_far}"
			);
			// Reject WITHOUT clearing the pending buffer below — an out-of-order
			// chunk must leave the already-received bytes intact so the client
			// can query the correct range and resume.
			return RegistryError::builder()
				.code(ErrorCode::BlobUploadInvalid)
				.message("Content-Range start does not match expected byte position")
				.status(StatusCode::RANGE_NOT_SATISFIABLE)
				.build()
				.into_result();
		}
	}

	// The chunk is in order — the loaded pending buffer is now being folded into
	// this request, so clear it from Redis. It is re-stored below if this
	// request again ends on a sub-threshold tail. (Deleting only after the
	// Content-Range check keeps an out-of-order chunk from destroying it.)
	let _ = redis.del(&pending_key).await;

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

	// Upload chunk to S3
	const CHUNK_FLUSH_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB - S3 requires all non-final parts to be at least this size

	let (mut updated_session, hasher, _, pending_size_after) =
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
					0u64,
				),
				async |(mut session, mut hasher, inflight_task, mut pending_size_after), chunk| {
					// If there's an inflight upload task from the previous chunk, await it and
					// update the session with the returned ETag and part number
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

					let part_number = session.uploaded_parts_etags.len() as i32 + 1;
					let chunk_size = chunk.len();
					let chunk_size_string =
						format!("{:.2}MB", chunk_size as f64 / (1024.0 * 1024.0));

					info!(
						"Uploading part {part_number} to S3 (buffered, {})",
						chunk_size_string
					);

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
							// The only way to get a chunk smaller than the flush threshold
							// is if it's the final chunk (the stream ended) — in that case
							// we can store that final chunk in Redis as a pending buffer to
							// be flushed on the next PATCH, which avoids violating S3's
							// minimum part size requirement for non-final parts. These bytes
							// aren't in S3 yet so they don't move `total_bytes_uploaded`; the
							// outgoing `Range:` header adds `pending_size_after` to reflect
							// bytes received (vs. bytes flushed to S3).
							info!("Storing {} pending buffer in Redis", chunk_size_string);
							redis
								.setex(
									&pending_key,
									constants::REGISTRY_BLOB_UPLOAD_PENDING_BUFFER_TTL.as_secs(),
									BASE64_STANDARD.encode(&chunk),
								)
								.await?;
							pending_size_after = chunk_size as u64;
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

					Ok((session, hasher, inflight_task, pending_size_after))
				},
			)
			.await?;

	updated_session.hasher_state = hex::encode(hasher.serialize());

	info!("All parts uploaded successfully");

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
			range: Range::new(0..updated_session.total_bytes_uploaded + pending_size_after)
				.map_err(|err| {
					error!("Invalid range error: {}", err);
					RegistryError::server_error(
						ErrorCode::BlobUploadInvalid,
						if cfg!(debug_assertions) {
							format!("invalid range specified: {}", err)
						} else {
							"invalid range specified".to_string()
						},
					)
				})?,
		})
		.body(Body::empty())
		.build()
		.into_result()
}
