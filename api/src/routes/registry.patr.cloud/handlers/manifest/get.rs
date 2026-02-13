//! GET manifest endpoint handler.
//!
//! This handler retrieves a manifest by tag or digest, streaming it from S3.
//! It supports content negotiation via the Accept header and returns
//! appropriate OCI-compliant headers.

use headers::{ContentLength, ContentType};
use tokio_util::io::ReaderStream;

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// GET manifest endpoint.
	///
	/// Retrieves a manifest by tag or digest from the registry.
	/// Supports content negotiation via Accept header.
	GetManifest,
	GET "/v2/{workspace_id}/{repo_name}/manifests/{reference}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The manifest reference (tag name or digest)
		#[preprocess(regex = constants::REGISTRY_TAG_OR_DIGEST_REGEX)]
		pub reference: String,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
	},
	response_headers = {
		/// The content type of the manifest
		pub content_type: ContentType,
		/// The digest of the manifest
		pub docker_content_digest: DockerContentDigest,
		/// The size of the manifest in bytes
		pub content_length: ContentLength,
	}
);

/// Handler for GET /v2/{workspace_id}/{repo_name}/manifests/{reference}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Resolves the reference (tag or digest) to a manifest digest
/// 4. Queries the database for manifest metadata
/// 5. Streams the manifest content from S3
/// 6. Returns with appropriate headers
pub async fn get_manifest(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					GetManifestPathProcessed {
						workspace_id,
						repo_name,
						reference,
					},
				query: (),
				headers: GetManifestRequestHeaders { authorization: _ },
				body: _,
			},
		database,
		redis: _,
		s3,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, GetManifestPath>,
) -> Result<RegistryResponse<GetManifestPath>, RegistryError> {
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

	let manifest_record = query!(
		r#"
		SELECT DISTINCT
			m.content_type,
			m.size,
			m.digest
		FROM
			container_registry_manifest m
		INNER JOIN
			container_registry_repository_manifest rm 
		ON
			m.digest = rm.manifest_digest
		INNER JOIN
			container_registry_repository r
		ON
			rm.repository_id = r.id
		WHERE
			(
				m.digest = $1 OR EXISTS(
					SELECT
						1
					FROM
						container_registry_repository_tag t
					WHERE
						t.repository_id = r.id AND
						t.manifest_digest = m.digest AND
						t.tag = $1
				)
			) AND
			r.workspace_id = $2 AND
			r.name = $3 AND
			r.deleted IS NULL;
		"#,
		reference,
		workspace_id as _,
		&repo_name
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or_else(|| {
		warn!(
			reference = %reference,
			repository = %repo_name,
			"Manifest not found"
		);
		RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message(format!("Manifest `{reference}` not found"))
			.code(ErrorCode::ManifestUnknown)
			.build()
	})?;

	info!("Found manifest in database");

	let object = s3
		.get_object()
		.bucket(&config.s3.bucket)
		.key(format!("manifests/{}", manifest_record.digest))
		.send()
		.await
		.inspect_err(|e| {
			error!("Failed to get manifest object in S3: {e}");
		})?;

	// Parse content type
	let content_type = manifest_record
		.content_type
		.parse()
		.unwrap_or_else(|_| ContentType::octet_stream());

	RegistryResponse::builder()
		.status_code(StatusCode::OK)
		.headers(GetManifestResponseHeaders {
			content_type,
			docker_content_digest: DockerContentDigest(manifest_record.digest),
			content_length: ContentLength(manifest_record.size.unsigned_abs()),
		})
		.body(Body::from_stream(ReaderStream::new(
			object.body.into_async_read(),
		)))
		.build()
		.into_result()
}

/// Create an S3 bucket client from configuration.
///
/// # Arguments
///
/// * `config` - S3 configuration
///
/// # Returns
///
/// An S3 Bucket client
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_docker_content_digest_header() {
		let digest = DockerContentDigest("sha256:abc123".into());
		assert_eq!(digest.0, "sha256:abc123");
	}
}
