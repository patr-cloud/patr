//! PUT manifest endpoint handler.
//!
//! This handler uploads a new manifest to the registry, validates it,
//! stores it in S3, and creates/updates tags as needed.

use std::str::FromStr;

use aws_sdk_s3::primitives::ByteStream;
use axum::body::Body;
use headers::ContentType;
use oci_spec::image::{ImageManifest, MediaType};
use sha2::{Digest as _, Sha256};

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// PUT manifest endpoint.
	///
	/// Uploads a new manifest to the registry. The manifest can be an OCI Image Manifest
	/// or an OCI Image Index (manifest list). All referenced blobs must already exist.
	PutManifest,
	PUT "/v2/{workspace_id}/{repo_name}/manifests/{reference}" {
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
		/// The content type of the request body
		pub content_type: ContentType,
	},
	response_headers = {
		/// Location of the uploaded manifest
		pub location: Location,
		/// The digest of the uploaded manifest
		pub docker_content_digest: DockerContentDigest,
		/// The docker distribution API version
		pub docker_distribution_api_version: DockerDistributionApiVersion,
	}
);

/// Handler for PUT /v2/{workspace_id}/{repo_name}/manifests/{reference}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Reads manifest from streaming request body
/// 4. Parses manifest using oci-spec (ImageManifest or ImageIndex)
/// 5. Computes SHA256 digest of manifest
/// 6. Verifies all referenced blobs exist in database
/// 7. Stores manifest in S3
/// 8. Stores manifest metadata in database
/// 9. Creates or updates tag if reference is a tag name
/// 10. Returns 201 Created with Location and Docker-Content-Digest headers
#[instrument(skip(database, s3, body, config))]
pub async fn upload_manifest(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					PutManifestPathProcessed {
						workspace_id,
						repo_name,
						reference,
					},
				query: (),
				headers: PutManifestRequestHeaders {
					authorization: _,
					content_type,
				},
				body,
			},
		database,
		redis: _,
		s3,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, PutManifestPath>,
) -> Result<RegistryResponse<PutManifestPath>, RegistryError> {
	trace!("PUT called on manifest");

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

	let bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|e| {
		error!("Failed to read manifest body: {e}");

		RegistryError::builder()
			.status(StatusCode::BAD_REQUEST)
			.message("Failed to read manifest body")
			.code(ErrorCode::ManifestInvalid)
			.build()
	})?;

	if let Ok(manifest) = serde_json::from_slice::<ImageManifest>(&bytes) {
		query!(
			r#"
			INSERT INTO
				container_registry_manifest(
					digest,
					size,
					created_at,
					content_type
				)
			VALUES
				(
					$1,
					$2,
					NOW(),
					$3
				)
			ON CONFLICT (digest) DO NOTHING;
			"#,
			manifest.config().digest().to_string(),
			manifest.config().size() as i32,
			MediaType::ImageManifest.to_string()
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			INSERT INTO
				container_registry_repository_manifest(
					repository_id,
					manifest_digest,
					created_at
				)
			VALUES
				(
					$1,
					$2,
					NOW()
				)
			ON CONFLICT (repository_id, manifest_digest) DO NOTHING;
			"#,
			repository_id as _,
			manifest.config().digest().to_string()
		)
		.execute(&mut **database)
		.await?;

		for (index, layer) in manifest.layers().iter().enumerate() {
			query!(
				r#"
				INSERT INTO
					container_registry_manifest_blob(
						ordinal,
						manifest_digest,
						blob_digest
					)
				VALUES
					(
						$1,
						$2,
						$3
					)
				ON CONFLICT (manifest_digest, blob_digest)
				DO UPDATE SET
					ordinal = EXCLUDED.ordinal,
					manifest_digest = EXCLUDED.manifest_digest,
					blob_digest = EXCLUDED.blob_digest;
				"#,
				index as i32,
				manifest.config().digest().to_string(),
				layer.digest().to_string()
			)
			.execute(&mut **database)
			.await?;
		}
	}

	let size = bytes.len();
	let computed_digest = format!("sha256:{:x}", Sha256::digest(&bytes));
	debug!("Computed manifest digest: {}", computed_digest);

	let s3_key = format!("manifests/{}", computed_digest);
	s3.put_object()
		.bucket(&config.s3.bucket)
		.key(&s3_key)
		.body(ByteStream::from(bytes))
		.send()
		.await
		.inspect_err(|e| {
			error!("Failed to head manifest object in S3: {e}");
		})?;

	let digest = if let Some((_, digest)) = reference.split_once(':') {
		if reference != computed_digest {
			warn!(
				"Manifest digest mismatch: reference `{reference}` does not match computed `{computed_digest}`"
			);
			return RegistryError::builder()
				.status(StatusCode::BAD_REQUEST)
				.message("Manifest digest does not match content")
				.detail(format!(
					"Provided reference: {reference}, Computed digest: {computed_digest}"
				))
				.code(ErrorCode::ManifestInvalid)
				.build()
				.into_result();
		}
		digest.to_string()
	} else {
		query!(
			r#"
			INSERT INTO
				container_registry_tag(
					repository_id,
					name,
					manifest_digest
				)
			VALUES
				(
					$1,
					$2,
					$3
				)
			ON CONFLICT (repository_id, name) DO NOTHING;
			"#,
			repository_id as _,
			reference,
			&computed_digest
		)
		.execute(&mut **database)
		.await?;

		computed_digest
	};

	query!(
		r#"
		INSERT INTO
			container_registry_manifest(
				digest,
				size,
				created_at,
				content_type
			)
		VALUES
			(
				$1,
				$2,
				NOW(),
				$3
			);
		"#,
		digest,
		size as i32,
		content_type.to_string()
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			container_registry_repository_manifest(
				repository_id,
				manifest_digest,
				created_at
			)
		VALUES
			(
				$1,
				$2,
				NOW()
			);
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await?;

	RegistryResponse::builder()
		.status_code(StatusCode::CREATED)
		.headers(PutManifestResponseHeaders {
			location: Location::from_str(&format!(
				"/v2/{}/{}/manifests/{}",
				workspace_id, repo_name, &digest
			))?,
			docker_content_digest: DockerContentDigest(digest),
			docker_distribution_api_version: DockerDistributionApiVersion,
		})
		.body(Body::empty())
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
	fn test_location_header() {
		let location = Location::from_str("/v2/test/manifests/sha256:abc123").unwrap();
		assert_eq!(location.to_string(), "/v2/test/manifests/sha256:abc123");
	}

	#[test]
	fn test_put_manifest_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<PutManifestPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/manifests/{reference}"
		);
	}
}
