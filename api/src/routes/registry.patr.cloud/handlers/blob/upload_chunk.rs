//! PATCH blob upload chunk endpoint handler.
//!
//! This handler processes chunked blob uploads, allowing clients to upload
//! large blobs in multiple parts using the OCI Distribution API's chunked
//! upload protocol.

use std::{self, str::FromStr, time::Duration};

use axum::body::Body;
use headers::{ContentRange, ContentType};
use http_body::Body as _;
use models::utils::{DockerUploadUuid, Range};
use rustis::commands::StringCommands;

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

	if body.is_end_stream() {
		warn!("Empty body provided for single blob upload");
		return RegistryError::builder()
			.code(ErrorCode::BlobUploadInvalid)
			.message("Body must not be empty for single blob upload")
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

	info!(
		"Uploading part {} to S3",
		session.uploaded_parts_etags.len() + 1
	);

	info!("Uploading bytes to S3");

	let response = s3
		.upload_part()
		.bucket(&config.s3.bucket)
		.key(&format!("uploads/{}", session_id))
		.upload_id(&session.upload_id)
		.part_number(session.uploaded_parts_etags.len() as i32 + 1)
		.body(BodyStreamWrapper::new(body.into_data_stream()).into_byte_stream())
		.send()
		.await?;

	info!("Uploaded part to S3 successfully");

	let content_length = s3
		.list_parts()
		.bucket(&config.s3.bucket)
		.key(&format!("uploads/{}", session_id))
		.upload_id(&session.upload_id)
		.max_parts(1)
		.part_number_marker(session.uploaded_parts_etags.len().to_string())
		.send()
		.await?
		.parts
		.and_then(|vec| vec.into_iter().next())
		.ok_or_else(|| {
			RegistryError::builder()
				.code(ErrorCode::Unsupported)
				.message("Failed to retrieve uploaded part information from S3")
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.build()
		})?
		.size
		.unwrap_or_default() as u64;

	info!("Retrieved uploaded part information from S3");

	let session = S3UploadSession {
		upload_id: session.upload_id,
		uploaded_parts_etags: {
			let mut etags = session.uploaded_parts_etags;

			etags.push(response.e_tag.ok_or_else(|| {
				RegistryError::builder()
					.code(ErrorCode::BlobUploadInvalid)
					.message("Missing or invalid ETag header in S3 response")
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.build()
			})?);

			etags
		},
		total_bytes_uploaded: session.total_bytes_uploaded + content_length as u64,
		initiated_by_login: session.initiated_by_login,
		initiated_by_ip: session.initiated_by_ip,
	};

	redis
		.setex(
			keys::registry_blob_upload_part_prefix(&session_id),
			Duration::from_hours(24).as_secs(),
			serde_json::to_string(&session)?,
		)
		.await?;

	RegistryResponse::builder()
		.status_code(StatusCode::ACCEPTED)
		.headers(UploadBlobChunkResponseHeaders {
			location: Location::from_str(&format!(
				"/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}"
			))?,
			docker_upload_uuid: DockerUploadUuid::new(session_id),
			range: Range::new(0..session.total_bytes_uploaded).map_err(|err| {
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
