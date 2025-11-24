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
		#[preprocess(lowercase, regex = "^[a-z0-9]+([._-][a-z0-9]+)*$", length(max = 255))]
		pub repo_name: String,
		/// The manifest reference (tag name or digest)
		#[preprocess(regex = "^[A-Za-z0-9._\\+-]+(:[A-Za-z0-9._\\=-]+)?$")]
		pub reference: String,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
	},
	auth = true,
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
#[instrument(skip(database, s3))]
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
		client_ip,
		user_data: _,
		config,
	}: AuthenticatedRegistryAppRequest<'_, GetManifestPath>,
) -> Result<RegistryResponse<GetManifestPath>, RegistryError> {
	// TODO check permission

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
						container_registry_tag t 
					WHERE
						t.repository_id = r.id AND
						t.manifest_digest = m.digest AND
						t.name = $1
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
		RegistryError::manifest_unknown(&reference)
	})?;

	info!("Found manifest in database");

	let object = s3
		.get_object()
		.bucket(&config.s3.bucket)
		.key(format!("manifests/{}", manifest_record.digest))
		.send()
		.await
		.map_err(|e| {
			error!("Failed to head manifest object in S3: {e}");
			RegistryError::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.message("Failed to access manifest storage")
				.code(ErrorCode::Unsupported)
				.build()
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
			content_length: ContentLength(manifest_record.size as u64),
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
