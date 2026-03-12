//! POST blob upload initiation endpoint handler.
//!
//! This handler initiates a blob upload session, which allows clients to upload
//! large blobs in chunks using the chunked upload protocol. It also supports
//! cross-repository blob mounting for efficient blob sharing.

use std::str::FromStr;

use axum::body::Body;
use headers::{ContentLength, ContentType};
use http_body::Body as _;
use models::utils::DockerUploadUuid;
use rustis::commands::StringCommands;
use sha2::{Digest as _, Sha256, digest::common::hazmat::SerializableState};

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// POST blob upload initiation endpoint.
	///
	/// Initiates a blob upload session for chunked uploads or handles
	/// cross-repository blob mounting.
	InitiateBlobUpload,
	POST "/v2/{workspace_id}/{repo_name}/blobs/uploads/" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
		/// Content-Length header, if provided
		pub content_length: ContentLength,
		/// Content-Type header, if provided
		pub content_type: OptionalHeader<ContentType>,
	},
	query = {
		/// Optional digest for cross-repository blob mounting
		pub mount: Option<String>,
		/// Optional source repository for blob mounting
		pub from: Option<String>,
		/// Optional digest for single blob upload
		pub digest: Option<String>,
	},
	response_headers = {
		/// Location header pointing to the upload URL
		pub location: Location,
		/// Docker-Upload-UUID header with the upload session UUID
		pub docker_upload_uuid: OptionalHeader<DockerUploadUuid>,
	}
);

/// Handler for POST /v2/{workspace_id}/{repo_name}/blobs/uploads/
///
/// This handler:
/// - Verifies workspace access
/// - Checks for mount query parameters (mount and from)
///     - If mount requested, handles cross-repository blob mounting
/// - Otherwise, creates new upload session with UUID
/// - Initiates S3 multipart upload
/// - Stores session in redis with S3 upload ID
/// - Returns 202 Accepted with Location, Docker-Upload-UUID, and Range headers
pub async fn initiate_upload(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: InitiateBlobUploadPathProcessed {
					workspace_id,
					repo_name,
				},
				query: InitiateBlobUploadQueryProcessed {
					mount,
					from,
					digest,
				},
				headers:
					InitiateBlobUploadRequestHeaders {
						authorization: _,
						content_length,
						content_type,
					},
				body,
			},
		database,
		redis,
		s3,
		client_ip,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, InitiateBlobUploadPath>,
) -> Result<RegistryResponse<InitiateBlobUploadPath>, RegistryError> {
	info!("POST blob upload initiation request");

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
		&mut **database,
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

	// Check for mount query parameters
	if let Some((_mount, _from)) = mount.zip(from) {
		info!("Attempting cross-repository blob mount");

		// Handle cross-repository blob mounting
		// If mount succeeds, return 201 Created
		// If mount fails, fall through to create new upload session
		// TODO: Should we check if the user has access to the source
		// repository?

		// match handle_blob_mount(&mut req, &repo_name, mount, from).await {
		// 	Ok(response) => return Ok(response),
		// 	Err(e) => {
		// 		warn!(
		// 			error = %e,
		// 			"Blob mount failed, falling back to new upload session"
		// 		);
		// 		// Fall through to create new upload session
		// 	}
		// }
	};

	let result = if let Some((digest, content_type)) = digest.zip(content_type.into_option()) {
		info!("Handling single blob upload initiation");

		// Handle single blob upload initiation
		if content_type != ContentType::octet_stream() {
			warn!(
				content_type = %content_type,
				"Invalid Content-Type for single blob upload"
			);
			return RegistryError::builder()
				.code(ErrorCode::BlobUploadInvalid)
				.message("Content-Type must be application/octet-stream for single blob upload")
				.status(StatusCode::BAD_REQUEST)
				.build()
				.into_result();
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
			content_length.0 as i64
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

		s3.put_object()
			.bucket(&config.s3.bucket)
			.key(format!("registry/blobs/{digest}"))
			.content_length(content_length.0 as i64)
			.content_type(content_type.to_string())
			.body(BodyStreamWrapper::new(body.into_data_stream()).into_byte_stream())
			.send()
			.await?;

		(
			StatusCode::CREATED,
			format!("/v2/{workspace_id}/{repo_name}/blobs/{digest}"),
			None,
		)
	} else {
		// Create new upload session with UUID
		let session_id = Uuid::new_v4();
		debug!("Generated new upload session ID: {session_id}");

		if content_length.0 != 0 {
			warn!(
				content_length = content_length.0,
				"Non-zero Content-Length provided for chunked upload initiation"
			);
			return RegistryError::builder()
				.code(ErrorCode::BlobUploadInvalid)
				.message("Content-Length must be zero for chunked upload initiation")
				.status(StatusCode::BAD_REQUEST)
				.build()
				.into_result();
		}

		info!("Initiated S3 multipart upload with session ID: {session_id}");

		// Initiate S3 multipart upload
		let response = s3
			.create_multipart_upload()
			.bucket(&config.s3.bucket)
			.key(format!("registry/uploads/{session_id}"))
			.send()
			.await?;

		let upload_id = response.upload_id().unwrap_or("");

		redis
			.setex(
				keys::registry_blob_upload_part_prefix(&session_id),
				constants::REGISTRY_BLOB_UPLOAD_SESSION_TTL.as_secs(),
				serde_json::to_string(&S3UploadSession {
					upload_id: upload_id.to_string(),
					uploaded_parts_etags: vec![],
					total_bytes_uploaded: 0,
					initiated_by_login: user_data.login_id,
					initiated_by_ip: client_ip,
					hasher_state: hex::encode(Sha256::new().serialize()),
				})?,
			)
			.await?;

		(
			StatusCode::ACCEPTED,
			format!("/v2/{workspace_id}/{repo_name}/blobs/uploads/{session_id}"),
			Some(session_id),
		)
	};

	let (status_code, location, session_id) = result;

	RegistryResponse::builder()
		.status_code(status_code)
		.headers(InitiateBlobUploadResponseHeaders {
			location: Location::from_str(&location)?,
			docker_upload_uuid: OptionalHeader::new(session_id.map(DockerUploadUuid::new)),
		})
		.body(Body::empty())
		.build()
		.into_result()
}
