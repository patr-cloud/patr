//! POST blob upload initiation endpoint handler.
//!
//! This handler initiates a blob upload session, which allows clients to upload
//! large blobs in chunks using the chunked upload protocol. It also supports
//! cross-repository blob mounting for efficient blob sharing.

use std::str::FromStr;

use aws_sdk_s3::{
	primitives::ByteStream,
	types::{CompletedMultipartUpload, CompletedPart},
};
use axum::body::Body;
use futures::TryStreamExt as _;
use headers::{ContentLength, ContentType};
use models::utils::DockerUploadUuid;
use oci_spec::image::{Digest as OciDigest, DigestAlgorithm};
use rustis::commands::StringCommands;
use sha2::{
	Digest as _,
	Sha256,
	Sha512,
	digest::{DynDigest, common::hazmat::SerializableState},
};

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// POST blob upload initiation endpoint.
	///
	/// Initiates a blob upload session for chunked uploads or handles
	/// cross-repository blob mounting.
	InitiateBlobUpload,
	POST "/v2/{workspace_id}/{repo_name}/blobs/uploads/" {
		/// The workspace ID
		#[cfg(feature = "cloud")]
		pub workspace_id: Uuid,
		/// The literal "registry" on self-hosted
		#[cfg(not(feature = "cloud"))]
		pub workspace_id: RegistryNamespace,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
		/// Content-Length header, if provided. Optional: a spec-compliant
		/// chunked-upload initiation (`POST` with no body) sends no
		/// Content-Length, so requiring it here rejected every multi-step
		/// upload before it began.
		pub content_length: OptionalHeader<ContentLength>,
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
	// Keep the original path segment around so the Location response header
	// echoes back what the client posted to (UUID on cloud, "registry" on
	// self-hosted) instead of leaking the resolved workspace UUID.
	let registry_namespace = workspace_id;

	#[cfg(not(feature = "cloud"))]
	let workspace_id = {
		let _ = workspace_id;
		query!(
			r#"
			SELECT
				id AS "id: Uuid"
			FROM
				workspace
			WHERE
				deleted IS NULL
			LIMIT 1;
			"#
		)
		.fetch_one(&mut **database)
		.await?
		.id
	};

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
		database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push),
	)
	.await;

	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, permission_id);

	if !authorized {
		debug!("User lacks push access to repository");
		// Workspace members get a clear 403 (they can already list repos via the
		// API, so there's nothing to hide); non-members get a 404 so outsiders
		// can't enumerate private repositories.
		return if user_data.workspaces.contains(&workspace_id) {
			RegistryError::builder()
				.status(StatusCode::FORBIDDEN)
				.message(format!(
					"You do not have push access to `{workspace_id}/{repo_name}`"
				))
				.code(ErrorCode::Denied)
				.build()
		} else {
			RegistryError::builder()
				.status(StatusCode::NOT_FOUND)
				.message("Repository not found")
				.code(ErrorCode::NameUnknown)
				.build()
		}
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

	// Normalize to `Option<u64>` — absent means the client sent no
	// Content-Length (valid for chunked-upload initiation).
	let content_length = content_length.into_option().map(|len| len.0);

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

		// Parse the claimed digest up front — a monolithic upload has no
		// PATCH/PUT round-trip, so this handler is the only place the content
		// can be verified against it.
		let reference_digest = OciDigest::from_str(&digest).map_err(|err| {
			warn!("Invalid digest `{digest}` for single blob upload: {err}");
			RegistryError::builder()
				.code(ErrorCode::DigestInvalid)
				.message("Invalid digest")
				.status(StatusCode::BAD_REQUEST)
				.build()
		})?;

		// A monolithic upload names its digest in the query, so — unlike the
		// chunked flow — we know the algorithm before streaming and can hash with
		// exactly it. Reject anything unsupported up front.
		let mut hasher: Box<dyn DynDigest + Send> = match reference_digest.algorithm() {
			DigestAlgorithm::Sha256 => Box::new(Sha256::new()),
			DigestAlgorithm::Sha512 => Box::new(Sha512::new()),
			other => {
				warn!("Unsupported digest algorithm `{other}` for single blob upload");
				return RegistryError::builder()
					.code(ErrorCode::Unsupported)
					.message("Unsupported digest algorithm")
					.status(StatusCode::BAD_REQUEST)
					.build()
					.into_result();
			}
		};

		// Stream the body into a temporary S3 multipart upload while hashing it
		let temp_key = format!("registry/uploads/{}", Uuid::new_v4());
		let upload_id = s3
			.create_multipart_upload()
			.bucket(&config.s3.bucket)
			.key(&temp_key)
			.send()
			.await?
			.upload_id
			.ok_or_else(|| {
				RegistryError::server_error(
					ErrorCode::BlobUploadInvalid,
					"S3 did not return an upload_id",
				)
			})?;

		// 5 MB: S3 requires every non-final multipart part to be at least this
		// large. `read_buffered_bytes` yields uniform 5 MB chunks plus a smaller
		// trailing one.
		const CHUNK_FLUSH_THRESHOLD: u64 = 5 * 1024 * 1024;
		let mut stream = std::pin::pin!(
			body.into_data_stream()
				.map_err(|err| {
					error!("Failed to read single blob upload body: {err}");
					RegistryError::builder()
						.code(ErrorCode::BlobUploadInvalid)
						.message("Failed to read request body")
						.status(StatusCode::BAD_REQUEST)
						.build()
				})
				.read_buffered_bytes(CHUNK_FLUSH_THRESHOLD)
		);

		let mut part_etags = Vec::<String>::new();
		let mut total_bytes = 0u64;
		while let Some(chunk) = stream.try_next().await? {
			hasher.update(&chunk);
			total_bytes += chunk.len() as u64;
			let part_number = part_etags.len() as i32 + 1;
			let response = s3
				.upload_part()
				.bucket(&config.s3.bucket)
				.key(&temp_key)
				.upload_id(&upload_id)
				.part_number(part_number)
				.content_length(chunk.len() as i64)
				.body(ByteStream::from(chunk))
				.send()
				.await?;
			part_etags.push(response.e_tag.ok_or_else(|| {
				RegistryError::server_error(
					ErrorCode::BlobUploadInvalid,
					"Missing ETag in S3 response",
				)
			})?);
		}

		// If the client declared a Content-Length, the streamed size must match it.
		if content_length.is_some_and(|len| len != total_bytes) {
			warn!(
				"Content-Length {} does not match body size {total_bytes}",
				content_length.unwrap_or_default()
			);
			let _ = s3
				.abort_multipart_upload()
				.bucket(&config.s3.bucket)
				.key(&temp_key)
				.upload_id(&upload_id)
				.send()
				.await;
			return RegistryError::builder()
				.code(ErrorCode::SizeInvalid)
				.message("Content-Length does not match the uploaded body size")
				.status(StatusCode::BAD_REQUEST)
				.build()
				.into_result();
		}

		// Verify the streamed content matches the claimed digest BEFORE committing it
		let computed_hex = hex::encode(hasher.finalize());
		if reference_digest.digest() != computed_hex {
			warn!(
				"Digest mismatch for single blob upload: expected {digest}, computed {computed_hex}"
			);
			let _ = s3
				.abort_multipart_upload()
				.bucket(&config.s3.bucket)
				.key(&temp_key)
				.upload_id(&upload_id)
				.send()
				.await;
			return RegistryError::builder()
				.code(ErrorCode::DigestInvalid)
				.message(format!(
					"Digest mismatch: expected {digest}, got {}:{computed_hex}",
					reference_digest.algorithm()
				))
				.status(StatusCode::BAD_REQUEST)
				.build()
				.into_result();
		}

		let blob_key = format!("registry/blobs/{digest}");
		if part_etags.is_empty() {
			// Zero-byte blob: S3 can't complete a 0-part multipart, so abort the
			// temp upload and write the empty object directly.
			let _ = s3
				.abort_multipart_upload()
				.bucket(&config.s3.bucket)
				.key(&temp_key)
				.upload_id(&upload_id)
				.send()
				.await;
			s3.put_object()
				.bucket(&config.s3.bucket)
				.key(&blob_key)
				.content_length(0)
				.content_type(content_type.to_string())
				.body(ByteStream::from_static(b""))
				.send()
				.await?;
		} else {
			// Complete the temp upload, overwrite the content-addressed blob with
			// it, then drop the temp.
			s3.complete_multipart_upload()
				.bucket(&config.s3.bucket)
				.key(&temp_key)
				.upload_id(&upload_id)
				.multipart_upload(
					CompletedMultipartUpload::builder()
						.set_parts(Some(
							part_etags
								.iter()
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
			s3.copy_object()
				.bucket(&config.s3.bucket)
				.copy_source(format!("{}/{temp_key}", config.s3.bucket))
				.key(&blob_key)
				.send()
				.await?;
			let _ = s3
				.delete_object()
				.bucket(&config.s3.bucket)
				.key(&temp_key)
				.send()
				.await;
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
			total_bytes as i64
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

		(
			StatusCode::CREATED,
			format!("/v2/{registry_namespace}/{repo_name}/blobs/{digest}"),
			None,
		)
	} else {
		// Create new upload session with UUID
		let session_id = Uuid::new_v4();
		debug!("Generated new upload session ID: {session_id}");

		if content_length.is_some_and(|len| len != 0) {
			warn!(
				content_length = content_length.unwrap_or(0),
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
			format!("/v2/{registry_namespace}/{repo_name}/blobs/uploads/{session_id}"),
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
