//! GET blob endpoint handler.
//!
//! This handler downloads a blob from the registry, streaming it directly from
//! S3. It supports HTTP range requests for partial downloads, which is useful
//! for resuming interrupted downloads or accessing specific parts of large
//! blobs.

use axum::body::Body;
use headers::{AcceptRanges, ContentLength, ContentType};
use rustis::commands::GenericCommands;
use tokio_util::io::ReaderStream;

use crate::{redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// GET blob endpoint.
	///
	/// Downloads a blob from the registry, streaming it directly from S3.
	/// Supports HTTP range requests for partial downloads.
	GetBlob,
	GET "/v2/{workspace_id}/{repo_name}/blobs/{digest}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The blob digest
		#[preprocess(regex = constants::REGISTRY_DIGEST_REGEX)]
		pub digest: String,
	},
	request_headers = {
		/// The Authorization header
		pub authorization: BearerToken,
		/// Optional Range header for partial downloads
		pub range: OptionalHeader<Range>,
	},
	response_headers = {
		/// The content type of the blob
		pub content_type: ContentType,
		/// The digest of the blob
		pub docker_content_digest: DockerContentDigest,
		/// The size of the blob in bytes (or range size)
		pub content_length: ContentLength,
		/// Accept-Ranges header to indicate range support
		pub accept_ranges: AcceptRanges,
	}
);

/// Handler for GET /v2/{workspace_id}/{repo_name}/blobs/{reference}
///
/// This handler:
/// - Verifies workspace access
/// - Queries the database for blob metadata
/// - Streams blob content from S3
/// - Supports HTTP range requests for partial downloads
/// - Returns with appropriate headers
pub async fn get_blob(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: GetBlobPathProcessed {
					workspace_id,
					repo_name,
					digest,
				},
				query: (),
				headers: GetBlobRequestHeaders {
					authorization: _,
					range,
				},
				body: _,
			},
		database,
		redis,
		s3,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, GetBlobPath>,
) -> Result<RegistryResponse<GetBlobPath>, RegistryError> {
	info!("GET blob request");

	// Check that the user can pull from this repository
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
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Pull)
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

	// Check if the blob is linked to this repo via a manifest (permanent)
	let exists_in_db = query!(
		r#"
		SELECT (
			/* Check if the blob is a layer in any manifest linked to this repo */
			EXISTS(
				SELECT
					1
				FROM
					container_registry_repository_manifest repo_manifest
				INNER JOIN
					container_registry_manifest_blob manifest_blob
				ON
					manifest_blob.manifest_digest = repo_manifest.manifest_digest
				WHERE
					repo_manifest.repository_id = $2 AND
					manifest_blob.blob_digest = $1
			)
			OR
			/* Check if the blob is a config for any manifest linked to this repo */
			EXISTS (
				SELECT
					1
				FROM
					container_registry_repository_manifest repo_manifest
				INNER JOIN
					container_registry_manifest manifest
				ON
					manifest.digest = repo_manifest.manifest_digest
				WHERE
					repo_manifest.repository_id = $2 AND
					manifest.config_blob_digest = $1
			)
		) AS "exists!";
		"#,
		digest,
		repository_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.exists;

	// Also check if the blob was recently uploaded to this repo (temporary Redis
	// key)
	let exists_in_redis = if !exists_in_db {
		redis
			.exists(keys::repository_for_registry_blob(&repository_id, &digest))
			.await? > 0
	} else {
		true
	};

	let exists = exists_in_db || exists_in_redis;

	if !exists {
		warn!("Blob not found");
		return RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Blob not found as a part of this repository")
			.code(ErrorCode::ManifestBlobUnknown)
			.build()
			.into_result();
	}

	info!("Found blob in database/redis");

	// Use S3 bucket from request (pre-initialized in AppState)
	let object = s3
		.get_object()
		.bucket(&config.s3.bucket)
		.key(format!("blobs/{digest}"))
		.set_range(range.into_option().map(|range| range.to_string()))
		.send()
		.await?;

	RegistryResponse::builder()
		.status_code(StatusCode::OK)
		.headers(GetBlobResponseHeaders {
			content_type: ContentType::octet_stream(),
			docker_content_digest: DockerContentDigest(digest),
			content_length: ContentLength(object.content_length.unwrap_or_default().unsigned_abs()),
			accept_ranges: AcceptRanges::bytes(),
		})
		.body(Body::from_stream(ReaderStream::new(
			object.body.into_async_read(),
		)))
		.build()
		.into_result()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_blob_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<GetBlobPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/{digest}"
		);
	}
}
