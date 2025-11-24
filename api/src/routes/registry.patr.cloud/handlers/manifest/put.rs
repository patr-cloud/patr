//! PUT manifest endpoint handler.
//!
//! This handler uploads a new manifest to the registry, validates it,
//! stores it in S3, and creates/updates tags as needed.

use axum::body::Body;
use futures::TryStreamExt;
use headers::ContentType;
use http::HeaderValue;
use oci_spec::image::{ImageIndex, ImageManifest};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use crate::routes::registry_patr_cloud::prelude::*;

/// Custom header for Location
#[derive(Debug, Clone, PartialEq)]
pub struct Location(String);

impl Location {
	/// Create a new Location header with the given URL
	pub fn new(url: impl Into<String>) -> Self {
		Self(url.into())
	}
}

impl headers::Header for Location {
	fn name() -> &'static headers::HeaderName {
		&http::header::LOCATION
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;
		let str_value = value.to_str().map_err(|_| headers::Error::invalid())?;
		Ok(Self(str_value.to_string()))
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		if let Ok(value) = HeaderValue::from_str(&self.0) {
			values.extend(std::iter::once(value));
		}
	}
}

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
		#[preprocess(lowercase, regex = "^[a-z0-9]+([._-][a-z0-9]+)*$", length(max = 255))]
		pub repo_name: String,
		/// The manifest reference (tag name or digest)
		#[preprocess(regex = "^[A-Za-z0-9._\\+-]+(:[A-Za-z0-9._\\=-]+)?$")]
		pub reference: String,
	},
	request_headers = {
		/// The authorization header
		pub authorization: BearerToken,
		/// The content type of the request body
		pub content_type: ContentType,
	},
	auth = true,
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
		client_ip,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, PutManifestPath>,
) -> Result<RegistryResponse<PutManifestPath>, RegistryError> {
	trace!("PUT called on get manifest");

	let workspace_id = workspace_id;
	let repository_id = check_repository(&repo_name, state.clone()).await?;

	// TODO check permission

	let body_bytes = to_bytes(body, usize::MAX)
		.await
		.inspect(|body| {
			trace!("body chunk size: {}", body.len());
		})
		.inspect_err(|error| {
			error!("Error reading body stream: {}", error);
		})
		.map_err(internal_server_error_response)?;

	let size = body_bytes.len();
	let body_stream = body_bytes.to_vec();

	let digest = if let Some((_, digest)) = reference.split_once(':') {
		digest.to_string()
	} else {
		let digest = format!("sha256:{:x}", Sha256::digest(&body_bytes));
		// Check if tag exists
		let tag_in_db = query!(
			r#"
			SELECT 
				*
			FROM
				container_registry_tag AS tag
			WHERE
				repository_id = $1 AND
				name = $2;
			"#,
			repository_id as _,
			tag
		)
		.fetch_optional(&mut **database)
		.await
		.map_err(internal_server_error_response)?;

		if tag_in_db.is_none() {
			query!(
				r#"
				INSERT INTO
					container_registry_tag(
						repository_id,
						name,
						manifest_digest
					) VALUES (
						$1,
						$2,
						$3
					);
				"#,
				repository_id as _,
				reference,
				digest
			)
			.execute(&mut **database)
			.await
			.map_err(internal_server_error_response)?;
		}

		digest
	};

	let s3_key = format!("manifests/{}", &digest);
	let response = s3
		.put_object()
		.bucket(&config.s3.bucket)
		.key(&s3_key)
		.body(body_stream.into())
		.send()
		.await
		.map_err(|e| {
			error!("Failed to head manifest object in S3: {e}");
			RegistryError::with_status(
				ErrorCode::ManifestInvalid,
				"Failed to push manifest to S3",
				StatusCode::BAD_REQUEST,
			)
		})?;

	query!(
		r#"
		INSERT INTO container_registry_manifest(
			digest,
			size,
			created_at,
			content_type
		) VALUES (
		 	$1,
			$2,
			NOW(),
			$3
		);
		"#,
		digest,
		size as i32,
		content_type
	)
	.execute(&mut **database)
	.await
	.map_err(internal_server_error_response)?;

	query!(
		r#"
		INSERT INTO container_registry_repository_manifest(
			repository_id,
			manifest_digest,
			created_at
		) VALUES (
			$1,
			$2,
			NOW()
		);
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut **database)
	.await
	.map_err(internal_server_error_response)?;

	RegistryResponse::builder()
		.status_code(StatusCode::CREATED)
		.headers(PutManifestResponseHeaders {
			location: Location::new(format!(
				"/v2/{}/{}/manifests/{}",
				workspace_id, repo_name, &digest
			)),
			docker_content_digest: DockerContentDigest(digest),
			docker_distribution_api_version: DockerDistributionApiVersion,
		})
		.body(Body::empty())
		.build()
		.into_result();

	// Read manifest from streaming request body
	debug!("Reading manifest from request body");

	// Convert the body stream to a stream of Result<Bytes, std::io::Error>
	let data_stream = body
		.into_data_stream()
		.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

	let mut manifest_bytes = Vec::new();
	StreamReader::new(data_stream)
		.read_to_end(&mut manifest_bytes)
		.await
		.map_err(|e| {
			error!("Failed to read manifest body: {}", e);
			RegistryError::manifest_invalid(format!("Failed to read manifest body: {}", e))
		})?;

	debug!(size = manifest_bytes.len(), "Read manifest bytes");

	// 4. Parse manifest using oci-spec
	// Try to detect content type from the manifest itself
	let content_type = detect_manifest_content_type(&manifest_bytes)?;

	debug!(content_type = %content_type, "Parsing manifest");

	// Try to parse as ImageManifest or ImageIndex based on content type
	let (manifest_type, referenced_blobs) = if content_type.contains("image.manifest") {
		// Parse as ImageManifest
		let manifest: ImageManifest = serde_json::from_slice(&manifest_bytes).map_err(|e| {
			error!("Failed to parse manifest as ImageManifest: {}", e);
			RegistryError::manifest_invalid(format!("Invalid manifest JSON: {}", e))
		})?;

		debug!("Parsed as ImageManifest");

		// Extract referenced blob digests
		let mut blobs = Vec::new();

		// Add config blob
		blobs.push(manifest.config().digest().to_string());

		// Add layer blobs
		for layer in manifest.layers() {
			blobs.push(layer.digest().to_string());
		}

		("ImageManifest", blobs)
	} else if content_type.contains("image.index") || content_type.contains("manifest.list") {
		// Parse as ImageIndex
		let index: ImageIndex = serde_json::from_slice(&manifest_bytes).map_err(|e| {
			error!("Failed to parse manifest as ImageIndex: {}", e);
			RegistryError::manifest_invalid(format!("Invalid manifest index JSON: {}", e))
		})?;

		debug!("Parsed as ImageIndex");

		// Extract referenced manifest digests
		let mut manifests = Vec::new();
		for manifest_descriptor in index.manifests() {
			manifests.push(manifest_descriptor.digest().to_string());
		}

		("ImageIndex", manifests)
	} else {
		error!("Unsupported content type: {}", content_type);
		return Err(RegistryError::unsupported(format!(
			"Unsupported manifest content type: {}",
			content_type
		)));
	};

	info!(
		manifest_type = manifest_type,
		referenced_count = referenced_blobs.len(),
		"Manifest parsed successfully"
	);

	// 5. Compute SHA256 digest of manifest
	let digest_bytes = Sha256::new().chain_update(&manifest_bytes).finalize();
	let manifest_digest = format!("sha256:{:x}", digest_bytes);

	info!(digest = %manifest_digest, "Computed manifest digest");

	// 6. Verify all referenced blobs exist in database
	debug!("Verifying referenced blobs exist");
	for blob_digest in &referenced_blobs {
		verify_blob_exists(database, blob_digest).await?;
	}
	info!("All referenced blobs verified");

	// 7. Store manifest in S3
	debug!("Storing manifest in S3");
	let s3_key = format!("manifests/{}", manifest_digest);
	upload_blob_to_s3(&s3, &s3_key, manifest_bytes.clone()).await?;
	info!(s3_key = %s3_key, "Manifest stored in S3");

	// 8. Store manifest metadata in database
	debug!("Storing manifest metadata in database");

	// First, ensure the repository exists
	let repository_id = ensure_repository_exists(database, workspace_id, &repo_name).await?;

	// Insert or update manifest record
	query!(
		r#"
		INSERT INTO container_registry_manifest (digest, size, created_at, content_type)
		VALUES ($1, $2, NOW(), $3)
		ON CONFLICT (digest) DO UPDATE
		SET content_type = EXCLUDED.content_type
		"#,
		manifest_digest,
		manifest_bytes.len() as i64,
		content_type
	)
	.execute(&mut **database)
	.await?;

	// Link manifest to repository
	query!(
		r#"
		INSERT INTO container_registry_repository_manifest (repository_id, manifest_digest, created_at)
		VALUES ($1, $2, NOW())
		ON CONFLICT (repository_id, manifest_digest) DO NOTHING
		"#,
		repository_id as _,
		manifest_digest
	)
	.execute(&mut **database)
	.await?;

	// Store manifest-to-blob relationships
	for (ordinal, blob_digest) in referenced_blobs.iter().enumerate() {
		query!(
			r#"
			INSERT INTO container_registry_layer_manifest (ordinal, manifest_digest, layer_blob_digest)
			VALUES ($1, $2, $3)
			ON CONFLICT (manifest_digest, layer_blob_digest) DO NOTHING
			"#,
			ordinal as i32,
			manifest_digest,
			blob_digest
		)
		.execute(&mut **database)
		.await?;
	}

	info!("Manifest metadata stored in database");

	// 9. Create or update tag if reference is a tag name (not a digest)
	if !reference.starts_with("sha256:") {
		debug!(tag = %reference, "Creating/updating tag");

		query!(
			r#"
			INSERT INTO container_registry_tag (name, repository_id, manifest_digest)
			VALUES ($1, $2, $3)
			ON CONFLICT (name, repository_id) DO UPDATE
			SET manifest_digest = EXCLUDED.manifest_digest
			"#,
			reference,
			repository_id as _,
			manifest_digest
		)
		.execute(&mut **database)
		.await?;

		info!(tag = %reference, "Tag created/updated");
	}

	// 10. Return 201 Created with Location and Docker-Content-Digest headers
	let location_url = format!(
		"/v2/{}/{}/manifests/{}",
		workspace_id, repo_name, manifest_digest
	);

	info!(
		digest = %manifest_digest,
		location = %location_url,
		"Manifest upload complete"
	);

	RegistryResponse::builder()
		.status_code(StatusCode::CREATED)
		.headers(PutManifestResponseHeaders {
			location: Location::new(location_url),
			docker_content_digest: DockerContentDigest(manifest_digest),
			docker_distribution_api_version: DockerDistributionApiVersion,
		})
		.body(Body::empty())
		.build()
		.into_result()
}

/// Detect the content type of a manifest by examining its structure.
///
/// # Arguments
///
/// * `manifest_bytes` - The raw manifest JSON bytes
///
/// # Returns
///
/// The detected content type string
///
/// # Errors
///
/// Returns `RegistryError` if the manifest cannot be parsed
fn detect_manifest_content_type(manifest_bytes: &[u8]) -> Result<String, RegistryError> {
	// Parse as generic JSON to check the structure
	let json: serde_json::Value = serde_json::from_slice(manifest_bytes).map_err(|e| {
		error!("Failed to parse manifest JSON: {}", e);
		RegistryError::manifest_invalid(format!("Invalid JSON: {}", e))
	})?;

	// Check for mediaType field
	if let Some(media_type) = json.get("mediaType").and_then(|v| v.as_str()) {
		return Ok(media_type.to_string());
	}

	// Check for schemaVersion and manifests array (indicates ImageIndex)
	if json.get("manifests").is_some() {
		return Ok("application/vnd.oci.image.index.v1+json".to_string());
	}

	// Check for schemaVersion and config (indicates ImageManifest)
	if json.get("config").is_some() {
		return Ok("application/vnd.oci.image.manifest.v1+json".to_string());
	}

	// Default to image manifest
	Ok("application/vnd.oci.image.manifest.v1+json".to_string())
}

/// Verify that a blob exists in the database.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `digest` - The blob digest to check
///
/// # Returns
///
/// Ok(()) if the blob exists
///
/// # Errors
///
/// Returns `RegistryError::manifest_blob_unknown` if the blob doesn't exist
async fn verify_blob_exists(
	database: &mut DatabaseTransaction,
	digest: &str,
) -> Result<(), RegistryError> {
	#[derive(Debug)]
	struct BlobExists {
		exists: Option<bool>,
	}

	let result: BlobExists = sqlx::query_as!(
		BlobExists,
		r#"
		SELECT EXISTS(
			SELECT 1 FROM container_registry_layer_blob WHERE digest = $1
		) as "exists"
		"#,
		digest
	)
	.fetch_one(&mut **database)
	.await?;

	if result.exists.unwrap_or(false) {
		Ok(())
	} else {
		warn!(digest = %digest, "Referenced blob not found");
		Err(RegistryError::manifest_blob_unknown(digest))
	}
}

/// Ensure a repository exists in the database, creating it if necessary.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `workspace_id` - The workspace ID
/// * `repo_name` - The repository name
///
/// # Returns
///
/// The repository ID (UUID)
///
/// # Errors
///
/// Returns `RegistryError` if database operations fail
async fn ensure_repository_exists(
	database: &mut DatabaseTransaction,
	workspace_id: Uuid,
	repo_name: &str,
) -> Result<Uuid, RegistryError> {
	#[derive(Debug)]
	struct RepositoryRecord {
		id: Uuid,
	}

	// Try to find existing repository
	let existing: Option<RepositoryRecord> = sqlx::query_as!(
		RepositoryRecord,
		r#"
		SELECT id
		FROM container_registry_repository
		WHERE workspace_id = $1 AND name = $2 AND deleted IS NULL
		"#,
		workspace_id as _,
		repo_name
	)
	.fetch_optional(&mut **database)
	.await?;

	if let Some(repo) = existing {
		debug!(repository_id = %repo.id, "Repository already exists");
		return Ok(repo.id);
	}

	// Repository doesn't exist, create it
	let repository_id = Uuid::new_v4();

	debug!(repository_id = %repository_id, "Creating new repository");

	sqlx::query!(
		r#"
		INSERT INTO container_registry_repository (id, workspace_id, name, created_at, updated_at, deleted)
		VALUES ($1, $2, $3, NOW(), NOW(), NULL)
		"#,
		repository_id as _,
		workspace_id as _,
		repo_name
	)
	.execute(&mut **database)
	.await?;

	info!(repository_id = %repository_id, "Repository created");

	Ok(repository_id)
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
		let location = Location::new("/v2/test/manifests/sha256:abc123");
		assert_eq!(location.0, "/v2/test/manifests/sha256:abc123");
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
