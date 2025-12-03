//! HEAD blob endpoint handler.
//!
//! This handler checks if a blob exists and returns metadata headers without
//! the blob body. It's used by clients to verify blob existence and get
//! size information before downloading.

use headers::{AcceptRanges, ContentLength, ContentType};

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// HEAD blob endpoint.
	///
	/// Checks if a blob exists and returns metadata headers without the body.
	/// Used for verifying blob existence and getting size information.
	HeadBlob,
	HEAD "/v2/{workspace_id}/{repo_name}/blobs/{digest}" {
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

/// Handler for HEAD "/v2/{workspace_id}/{repo_name}/blobs/{digest}"
///
/// This handler:
/// - Verifies workspace access
/// - Queries the database for blob metadata
/// - Returns headers with Content-Length and Docker-Content-Digest
pub async fn head_blob(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: HeadBlobPathProcessed {
					workspace_id,
					repo_name,
					digest,
				},
				query: (),
				headers: HeadBlobRequestHeaders {
					authorization: _,
					range,
				},
				body: _,
			},
		database,
		redis: _,
		s3,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, HeadBlobPath>,
) -> Result<RegistryResponse<HeadBlobPath>, RegistryError> {
	info!("HEAD blob request");

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

	let exists = query!(
		r#"
		SELECT EXISTS(
			SELECT
				1
			FROM
				container_registry_repository
			INNER JOIN
				container_registry_repository_manifest
			ON
				container_registry_repository.id = container_registry_repository_manifest.repository_id
			INNER JOIN
				container_registry_manifest_blob
			ON
				container_registry_repository_manifest.manifest_digest = container_registry_manifest_blob.manifest_digest
			WHERE
				container_registry_manifest_blob.blob_digest = $1 AND
				container_registry_repository.workspace_id = $2 AND
				container_registry_repository.name = $3 AND
				container_registry_repository.deleted IS NULL
		) AS "exists!";
		"#,
		digest,
		workspace_id as _,
		&repo_name,
	)
	.fetch_one(&mut **database)
	.await?
	.exists;

	if !exists {
		warn!("Blob not found");
		return RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message("Blob not found as a part of this repository")
			.code(ErrorCode::ManifestBlobUnknown)
			.build()
			.into_result();
	}

	info!("Found blob in database");

	// Use S3 bucket from request (pre-initialized in AppState)
	let object = s3
		.head_object()
		.bucket(&config.s3.bucket)
		.key(format!("blobs/{digest}"))
		.set_range(range.into_option().map(|range| range.to_string()))
		.send()
		.await?;

	RegistryResponse::builder()
		.status_code(StatusCode::OK)
		.headers(HeadBlobResponseHeaders {
			content_type: ContentType::octet_stream(),
			docker_content_digest: DockerContentDigest(digest),
			content_length: ContentLength(object.content_length.unwrap_or_default() as u64),
			accept_ranges: AcceptRanges::bytes(),
		})
		.body(Body::empty())
		.build()
		.into_result()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_head_blob_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<HeadBlobPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/{digest}"
		);
	}
}
