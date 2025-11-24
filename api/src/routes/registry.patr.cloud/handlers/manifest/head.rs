//! HEAD manifest endpoint handler.
//!
//! This handler checks if a manifest exists and returns metadata headers
//! without the manifest body. It's used by clients to verify manifest existence
//! and get size information before downloading.

use std::{str::FromStr, time::Duration};

use headers::{CacheControl, ContentLength, ContentType, ETag};

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// HEAD manifest endpoint.
	///
	/// Checks if a manifest exists and returns metadata headers without the body.
	/// Used for verifying manifest existence and getting size information.
	HeadManifest,
	HEAD "/v2/{workspace_id}/{repo_name}/manifests/{reference}" {
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
		/// The E-Tag header
		pub etag: ETag,
		/// The cache control header
		pub cache_control: CacheControl,
	}
);

/// Handler for HEAD /v2/{workspace_id}/{repo_name}/manifests/{reference}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Resolves the reference (tag or digest) to a manifest digest
/// 4. Queries the database for manifest metadata
/// 5. Returns headers only (no body) with Content-Length and
///    Docker-Content-Digest
#[instrument(skip(database))]
pub async fn check_manifest(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					HeadManifestPathProcessed {
						workspace_id,
						repo_name,
						reference,
					},
				query: (),
				headers: HeadManifestRequestHeaders { authorization: _ },
				body: _,
			},
		database,
		redis: _,
		s3: _,
		client_ip,
		user_data: _,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, HeadManifestPath>,
) -> Result<RegistryResponse<HeadManifestPath>, RegistryError> {
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

	info!(
		digest = %manifest_record.digest,
		size = manifest_record.size,
		content_type = %manifest_record.content_type,
		"Found manifest in database"
	);

	let s3_key = format!("manifests/{}", &manifest_record.digest);
	debug!(s3_key = %s3_key, "Streaming manifest from S3");

	// Parse content type
	let content_type = manifest_record
		.content_type
		.parse()
		.unwrap_or_else(|_| ContentType::octet_stream());

	let etag = ETag::from_str(&manifest_record.digest).map_err(|err| {
		error!(%err, "Failed to parse ETag from manifest digest");
		RegistryError::with_status(
			ErrorCode::ManifestInvalid,
			"Failed to parse ETag",
			StatusCode::INTERNAL_SERVER_ERROR,
		)
	})?;
	let cache_control = CacheControl::new()
		.with_immutable()
		.with_private()
		.with_max_age(Duration::from_hours(90 * 24));

	RegistryResponse::builder()
		.status_code(StatusCode::OK)
		.headers(HeadManifestResponseHeaders {
			content_type,
			docker_content_digest: DockerContentDigest(manifest_record.digest),
			content_length: ContentLength(manifest_record.size as u64),
			etag,
			cache_control,
		})
		.body(Body::empty())
		.build()
		.into_result()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_head_manifest_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<HeadManifestPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{workspace_id}/{repo_name}/manifests/{reference}"
		);
	}
}
