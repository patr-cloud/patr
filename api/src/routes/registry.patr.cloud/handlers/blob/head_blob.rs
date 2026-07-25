//! HEAD blob endpoint handler.
//!
//! This handler checks if a blob exists and returns metadata headers without
//! the blob body. It's used by clients to verify blob existence and get
//! size information before downloading.

use headers::{AcceptRanges, ContentLength, ContentType};
use rustis::commands::GenericCommands as _;

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// HEAD blob endpoint.
	///
	/// Checks if a blob exists and returns metadata headers without the body.
	/// Used for verifying blob existence and getting size information.
	HeadBlob,
	HEAD "/v2/{workspace_id}/{repo_name}/blobs/{digest}" {
		/// The workspace ID
		#[cfg(feature = "cloud")]
		pub workspace_id: Uuid,
		/// The literal "registry" on self-hosted
		#[cfg(not(feature = "cloud"))]
		pub workspace_id: RegistryNamespace,
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
		redis,
		s3,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, HeadBlobPath>,
) -> Result<RegistryResponse<HeadBlobPath>, RegistryError> {
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

	info!("HEAD blob request");

	// Check that the user can pull from this repository
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

	// A push HEADs each blob to check existence before uploading, so a push-only
	// token must be allowed to run this read: authorize pull OR push.
	let pull_permission_id = permissions::get_permission_id(
		database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Pull),
	)
	.await;
	let push_permission_id = permissions::get_permission_id(
		database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push),
	)
	.await;
	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, pull_permission_id) ||
			user_data.has_permission_on_resource(
				workspace_id,
				repository_id,
				push_permission_id,
			);

	if !authorized {
		debug!("User lacks pull access to repository");
		// Workspace members get a clear 403 (they can already list repos via the
		// API, so there's nothing to hide); non-members get a 404 so outsiders
		// can't enumerate private repositories.
		return if user_data.permissions.contains_key(&workspace_id) {
			RegistryError::builder()
				.status(StatusCode::FORBIDDEN)
				.message(format!(
					"You do not have pull access to `{workspace_id}/{repo_name}`"
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
					container_registry_manifest_layer layer
				ON
					layer.manifest_digest = repo_manifest.manifest_digest
				WHERE
					repo_manifest.repository_id = $2 AND
					layer.blob_digest = $1
			)
			OR
			/* Check if the blob is an image config for any manifest linked to this repo */
			EXISTS (
				SELECT
					1
				FROM
					container_registry_repository_manifest repo_manifest
				INNER JOIN
					container_registry_manifest_image image
				ON
					image.manifest_digest = repo_manifest.manifest_digest
				WHERE
					repo_manifest.repository_id = $2 AND
					image.config_blob_digest = $1
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
		.head_object()
		.bucket(&config.s3.bucket)
		.key(format!("registry/blobs/{digest}"))
		.set_range(range.into_option().map(|range| range.to_string()))
		.send()
		.await?;

	RegistryResponse::builder()
		.status_code(StatusCode::OK)
		.headers(HeadBlobResponseHeaders {
			content_type: ContentType::octet_stream(),
			docker_content_digest: DockerContentDigest(digest),
			content_length: ContentLength(object.content_length.unwrap_or_default().unsigned_abs()),
			accept_ranges: AcceptRanges::bytes(),
		})
		.body(Body::empty())
		.build()
		.into_result()
}
