//! PUT manifest endpoint handler.
//!
//! This handler uploads a new manifest to the registry, validates it,
//! stores it in S3, and creates/updates tags as needed.

use std::str::FromStr;

use aws_sdk_s3::primitives::ByteStream;
use axum::body::Body;
use headers::ContentType;
use oci_spec::image::{Digest, ImageConfiguration, ImageIndex, ImageManifest};
use rustis::commands::GenericCommands;
use sha2::{Digest as _, Sha256};

use crate::{redis::keys, routes::registry_patr_cloud::prelude::*};

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
/// 1. Verifies user has push access to the repository
/// 2. Reads and computes the SHA256 digest of the manifest body
/// 3. Validates the digest against the reference (if reference is a digest)
/// 4. Stores the manifest in S3
/// 5. Records manifest metadata and repository linkage in the database
/// 6. If the manifest is a valid OCI ImageManifest, records config and layer
///    blobs
/// 7. Creates or updates a tag if the reference is a tag name
/// 8. Returns 201 Created with Location and Docker-Content-Digest headers
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
		redis,
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

	// Read request body and compute digest
	let bytes = axum::body::to_bytes(body, constants::MAX_REGISTRY_MANIFEST_SIZE)
		.await
		.map_err(|e| {
			error!("Failed to read manifest body: {e}");

			RegistryError::builder()
				.status(StatusCode::BAD_REQUEST)
				.message("Failed to read manifest body")
				.code(ErrorCode::ManifestInvalid)
				.build()
		})?;
	let size = bytes.len();

	let reference_digest = Digest::from_str(&reference).ok();
	// Make sure the digest of the manifest matches the reference if the reference
	// is a digest. If the reference is a tag, we'll compute the digest and store it
	// under that tag.
	let digest = {
		let computed_digest = hex::encode(Sha256::digest(&bytes));

		debug!("Computed manifest digest: {computed_digest}");

		let digest_mismatch = reference_digest
			.as_ref()
			.map(|digest| digest.digest() != computed_digest)
			.unwrap_or(false);

		if digest_mismatch {
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

		format!("sha256:{computed_digest}")
	};

	let content_type = content_type.to_string();
	let inserted_new = match content_type.as_str() {
		"application/vnd.oci.image.manifest.v1+json" |
		"application/vnd.docker.distribution.manifest.v2+json" => {
			// Process the manifest as an OCI Image Manifest.
			let Ok(manifest) = serde_json::from_slice::<ImageManifest>(&bytes) else {
				warn!("Failed to parse manifest as OCI Image Manifest");
				return RegistryError::builder()
					.status(StatusCode::BAD_REQUEST)
					.message("Failed to parse manifest as OCI Image Manifest")
					.code(ErrorCode::ManifestInvalid)
					.build()
					.into_result();
			};

			let config_digest = manifest.config().digest().to_string();
			let config_blob = serde_json::from_slice::<ImageConfiguration>(
				&s3.get_object()
					.bucket(&config.s3.bucket)
					.key(format!("blobs/{config_digest}"))
					.send()
					.await?
					.body
					.collect()
					.await?
					.into_bytes(),
			)?;
			let platform = manifest
				.config()
				.platform()
				.as_ref()
				.map(|p| format!("{}/{}", p.os(), p.architecture()))
				.unwrap_or_else(|| format!("{}/{}", config_blob.os(), config_blob.architecture()));

			let inserted = query!(
				r#"
				INSERT INTO
					container_registry_manifest(
						digest,
						content_type,
						size,
						config_blob_digest,
						platform
					)
				VALUES
					(
						$1,
						$2,
						$3,
						$4,
						$5
					)
				ON CONFLICT (digest) DO NOTHING
				RETURNING digest;
				"#,
				digest,
				content_type,
				size as i32,
				config_digest,
				platform,
			)
			.fetch_optional(&mut **database)
			.await?
			.is_some();

			// Record each layer blob and clean up temporary Redis associations
			for layer in manifest.layers() {
				let blob_digest = layer.digest().to_string();

				query!(
					r#"
					INSERT INTO
						container_registry_manifest_blob(
							manifest_digest,
							blob_digest
						)
					VALUES
						($1, $2)
					ON CONFLICT (manifest_digest, blob_digest) DO NOTHING;
					"#,
					digest,
					&blob_digest
				)
				.execute(&mut **database)
				.await?;

				// The blob is now permanently linked via the manifest, so we can
				// remove the temporary Redis blob->repo association.
				let _ = redis
					.del(keys::repository_for_registry_blob(
						&repository_id,
						&blob_digest,
					))
					.await;
			}

			inserted
		}
		"application/vnd.oci.image.index.v1+json" |
		"application/vnd.docker.distribution.manifest.list.v2+json" => {
			// Process the manifest as an OCI Image Index (manifest list).
			let Ok(index) = serde_json::from_slice::<ImageIndex>(&bytes) else {
				warn!("Failed to parse manifest as OCI Image Index");
				return RegistryError::builder()
					.status(StatusCode::BAD_REQUEST)
					.message("Failed to parse manifest as OCI Image Index")
					.code(ErrorCode::ManifestInvalid)
					.build()
					.into_result();
			};

			let inserted = query!(
				r#"
				INSERT INTO
					container_registry_manifest(
						digest,
						content_type,
						size,
						config_blob_digest,
						platform
					)
				VALUES
					(
						$1,
						$2,
						$3,
						NULL,
						NULL
					)
				ON CONFLICT (digest) DO NOTHING
				RETURNING digest;
				"#,
				digest,
				content_type,
				size as i32,
			)
			.fetch_optional(&mut **database)
			.await?
			.is_some();

			// Record manifest references if this is an index
			for manifest in index.manifests() {
				let referenced_digest = manifest.digest().to_string();

				query!(
					r#"
					INSERT INTO
						container_registry_manifest_reference(
							digest,
							referenced_digest
						)
					VALUES
						($1, $2)
					ON CONFLICT (digest, referenced_digest) DO NOTHING;
					"#,
					digest,
					&referenced_digest
				)
				.execute(&mut **database)
				.await?;
			}

			inserted
		}
		_ => {
			warn!("Unsupported manifest content type: {content_type}");
			return RegistryError::builder()
				.status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
				.message("Unsupported manifest content type")
				.detail(format!("Content-Type `{content_type}` is not supported"))
				.code(ErrorCode::ManifestInvalid)
				.build()
				.into_result();
		}
	};

	if inserted_new {
		// If the manifest was newly inserted, we need to upload it to S3. If it
		// already exists, we can skip the S3 upload since the content is
		// immutable and must be identical.
		s3.put_object()
			.bucket(&config.s3.bucket)
			.key(format!("manifests/{digest}"))
			.content_type(content_type)
			.body(ByteStream::from(bytes))
			.send()
			.await
			.inspect_err(|e| {
				error!("Failed to upload manifest to S3: {e}");
			})?;
	}

	// Link this manifest to the repository
	query!(
		r#"
		INSERT INTO
			container_registry_repository_manifest(
				repository_id,
				manifest_digest,
				created_at
			)
		VALUES
			($1, $2, NOW())
		ON CONFLICT (repository_id, manifest_digest) DO NOTHING;
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await?;

	// Create or update tag if the reference is a tag name
	if reference_digest.is_none() {
		query!(
			r#"
			INSERT INTO
				container_registry_repository_tag(
					name,
					repository_id,
					manifest_digest,
					last_updated
				)
			VALUES
				($1, $2, $3, NOW())
			ON CONFLICT (repository_id, name)
			DO UPDATE SET
				manifest_digest = EXCLUDED.manifest_digest,
				last_updated = EXCLUDED.last_updated;
			"#,
			reference,
			repository_id as _,
			&digest
		)
		.execute(&mut **database)
		.await?;
	}

	// Return 201 Created with location and digest headers
	RegistryResponse::builder()
		.status_code(StatusCode::CREATED)
		.headers(PutManifestResponseHeaders {
			location: Location::from_str(&format!(
				"/v2/{workspace_id}/{repo_name}/manifests/{digest}"
			))?,
			docker_content_digest: DockerContentDigest(digest),
			docker_distribution_api_version: DockerDistributionApiVersion,
		})
		.body(Body::empty())
		.build()
		.into_result()
}
