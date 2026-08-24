//! HEAD manifest endpoint handler.
//!
//! This handler checks if a manifest exists and returns metadata headers
//! without the manifest body. It's used by clients to verify manifest existence
//! and get size information before downloading.

use std::{str::FromStr, time::Duration};

use headers::{CacheControl, ContentLength, ContentType, ETag};

use crate::{models::permissions, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// HEAD manifest endpoint.
	///
	/// Checks if a manifest exists and returns metadata headers without the body.
	/// Used for verifying manifest existence and getting size information.
	HeadManifest,
	HEAD "/v2/{workspace_id}/{repo_name}/manifests/{reference}" {
		/// The workspace ID
		#[cfg(feature = "cloud")]
		pub workspace_id: Uuid,
		/// The literal "registry" on self-hosted
		#[cfg(not(feature = "cloud"))]
		pub workspace_id: RegistryNamespace,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The manifest reference (tag name or digest)
		// Intentionally not regex-validated: an unknown or malformed reference
		// must reach the handler and 404 (ManifestUnknown), not 400 at preprocess
		// — strict validation here breaks OCI conformance (nonexistent → 404).
		#[preprocess(length(max = 255))]
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
		client_ip: _,
		user_data,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, HeadManifestPath>,
) -> Result<RegistryResponse<HeadManifestPath>, RegistryError> {
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

	let permission_id = permissions::get_permission_id(
		database,
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Pull),
	)
	.await;

	let authorized =
		user_data.has_permission_on_resource(workspace_id, repository_id, permission_id);

	if !authorized {
		debug!("User lacks pull access to repository");
		// Workspace members get a clear 403 (they can already list repos via the
		// API, so there's nothing to hide); non-members get a 404 so outsiders
		// can't enumerate private repositories.
		return if user_data.workspaces.contains(&workspace_id) {
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

	let manifest_record = query!(
		r#"
		SELECT DISTINCT
			m.media_type AS content_type,
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
				m.digest = $1 OR m.digest = (
					SELECT
						t.manifest_digest
					FROM
						container_registry_repository_tag t
					WHERE
						t.repository_id = r.id AND
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
		warn!("Manifest not found");
		RegistryError::builder()
			.status(StatusCode::NOT_FOUND)
			.message(format!("Manifest `{reference}` not found"))
			.code(ErrorCode::ManifestUnknown)
			.build()
	})?;

	info!("Found manifest in database");

	let s3_key = format!("registry/manifests/{}", &manifest_record.digest);
	debug!(s3_key = %s3_key, "Streaming manifest from S3");

	// Parse content type
	let content_type = manifest_record
		.content_type
		.parse()
		.unwrap_or_else(|_| ContentType::octet_stream());

	let etag = ETag::from_str(&format!("\"{}\"", manifest_record.digest)).map_err(|err| {
		error!("Failed to parse ETag from manifest digest: {err}");
		RegistryError::builder()
			.code(ErrorCode::ManifestInvalid)
			.message("Failed to parse ETag")
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.build()
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
			content_length: ContentLength(manifest_record.size.unsigned_abs()),
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
