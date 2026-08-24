//! GET blob endpoint handler.
//!
//! This handler downloads a blob from the registry, streaming it directly from
//! S3. It supports HTTP range requests for partial downloads, which is useful
//! for resuming interrupted downloads or accessing specific parts of large
//! blobs.

use axum::body::Body;
use headers::{AcceptRanges, ContentLength, ContentRange, ContentType, Header as _};
use rustis::commands::GenericCommands;
use tokio_util::io::ReaderStream;

use crate::{models::permissions, redis::keys, routes::registry_patr_cloud::prelude::*};

macros::declare_registry_endpoint!(
	/// GET blob endpoint.
	///
	/// Downloads a blob from the registry, streaming it directly from S3.
	/// Supports HTTP range requests for partial downloads.
	GetBlob,
	GET "/v2/{workspace_id}/{repo_name}/blobs/{digest}" {
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
		/// Content-Range header, present only on a 206 partial response
		pub content_range: OptionalHeader<ContentRange>,
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

	info!("GET blob request");

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

	// Forward the client's Range to S3/MinIO, which does the range math and
	// returns the partial slice + a Content-Range. `object.content_length` is
	// already the slice length when a range is honored.
	let range = range.into_option();
	let object = s3
		.get_object()
		.bucket(&config.s3.bucket)
		.key(format!("registry/blobs/{digest}"))
		.set_range(range.as_ref().map(|range| range.to_string()))
		.send()
		.await;

	let object = match object {
		Ok(object) => object,
		// MinIO answers an unsatisfiable range with 416. Surface that as 416
		// rather than the blanket 500 the generic `From<SdkError>` produces.
		Err(err)
			if err
				.raw_response()
				.map(|response| response.status().as_u16()) ==
				Some(416) =>
		{
			return RegistryError::builder()
				.status(StatusCode::RANGE_NOT_SATISFIABLE)
				.code(ErrorCode::BlobUnknown)
				.message("Requested range not satisfiable")
				.build()
				.into_result();
		}
		Err(err) => return Err(err.into()),
	};

	// If a range was requested and honored, respond 206 Partial Content with the
	// Content-Range MinIO computed (e.g. `bytes 0-499/1024`); otherwise a normal
	// 200 with the full blob.
	let content_range = object.content_range().and_then(|value| {
		let header_value = http::HeaderValue::from_str(value).ok()?;
		ContentRange::decode(&mut std::iter::once(&header_value)).ok()
	});
	let status_code = if range.is_some() && content_range.is_some() {
		StatusCode::PARTIAL_CONTENT
	} else {
		StatusCode::OK
	};

	RegistryResponse::builder()
		.status_code(status_code)
		.headers(GetBlobResponseHeaders {
			content_type: ContentType::octet_stream(),
			docker_content_digest: DockerContentDigest(digest),
			content_length: ContentLength(object.content_length.unwrap_or_default().unsigned_abs()),
			accept_ranges: AcceptRanges::bytes(),
			content_range: OptionalHeader::new(content_range),
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
			"/v2/{workspace_id}/{repo_name}/blobs/{digest}"
		);
	}
}
