//! HEAD blob endpoint handler.
//!
//! This handler checks if a blob exists and returns metadata headers without
//! the blob body. It's used by clients to verify blob existence and get
//! size information before downloading.

use http::HeaderValue;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		AuthenticatedRegistryRequest,
		RegistryEndpoint,
		RegistryError,
		RegistryResponse,
		types::RepositoryName,
		utils::repository::verify_workspace_access,
	},
};

/// Custom header for Docker content digest
#[derive(Debug, Clone, PartialEq)]
pub struct DockerContentDigest(String);

impl DockerContentDigest {
	pub fn new(digest: String) -> Self {
		Self(digest)
	}
}

impl headers::Header for DockerContentDigest {
	fn name() -> &'static headers::HeaderName {
		static NAME: headers::HeaderName =
			headers::HeaderName::from_static("docker-content-digest");
		&NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;
		let digest = value
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.to_string();
		Ok(Self(digest))
	}

	fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
		if let Ok(value) = HeaderValue::from_str(&self.0) {
			values.extend(std::iter::once(value));
		}
	}
}

macros::declare_registry_endpoint!(
	/// HEAD blob endpoint.
	///
	/// Checks if a blob exists and returns metadata headers without the body.
	/// Used for verifying blob existence and getting size information.
	HeadBlob,
	HEAD "/v2/{name}/blobs/{digest}" {
		/// The repository name in the format workspace_id/repo_name
		pub name: String,
		/// The blob digest (sha256:...)
		pub digest: String,
	},
	auth = true,
	response_headers = {
		/// The content type of the blob
		pub content_type: headers::ContentType,
		/// The digest of the blob
		pub docker_content_digest: DockerContentDigest,
		/// The size of the blob in bytes
		pub content_length: headers::ContentLength,
	}
);

/// Handler for HEAD /v2/{name}/blobs/{digest}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Validates digest format
/// 4. Queries the database for blob metadata
/// 5. Returns headers with Content-Length and Docker-Content-Digest
///
/// # Requirements
/// - 9.2: Return headers with Content-Length and Docker-Content-Digest
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, HeadBlobPath>,
) -> Result<RegistryResponse<HeadBlobPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		digest = %req.path.digest,
		user_id = %req.user_data.id,
		"HEAD blob request"
	);

	// 1. Parse repository name
	let repo_name = RepositoryName::parse(&req.path.name)?;
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		"Parsed repository name"
	);

	// 2. Verify workspace access
	verify_workspace_access(&req.user_data, repo_name.workspace_id())?;
	debug!("Workspace access verified");

	// 3. Validate digest format
	if !req.path.digest.starts_with("sha256:") {
		warn!(
			digest = %req.path.digest,
			"Invalid digest format"
		);
		return Err(RegistryError::digest_invalid(&req.path.digest));
	}
	debug!("Digest format validated");

	// 4. Query database for blob metadata
	#[derive(Debug)]
	struct BlobRecord {
		digest: String,
		size: i64,
	}

	let blob_record: BlobRecord = sqlx::query_as!(
		BlobRecord,
		r#"
		SELECT 
			b.digest,
			b.size
		FROM container_registry_layer_blob b
		INNER JOIN container_registry_layer_manifest lm 
			ON b.digest = lm.layer_blob_digest
		INNER JOIN container_registry_repository_manifest rm 
			ON lm.manifest_digest = rm.manifest_digest
		INNER JOIN container_registry_repository r 
			ON rm.repository_id = r.id
		WHERE b.digest = $1
			AND r.workspace_id = $2
			AND r.name = $3
			AND r.deleted IS NULL
		LIMIT 1
		"#,
		req.path.digest,
		repo_name.workspace_id() as _,
		repo_name.name()
	)
	.fetch_optional(&mut **req.database)
	.await?
	.ok_or_else(|| {
		warn!(
			digest = %req.path.digest,
			repository = %req.path.name,
			"Blob not found"
		);
		RegistryError::blob_unknown(&req.path.digest)
	})?;

	info!(
		digest = %blob_record.digest,
		size = blob_record.size,
		"Found blob in database"
	);

	// 5. Return response with headers only (no body)
	info!("Returning blob metadata headers");

	// ContentType for octet-stream
	let content_type = headers::ContentType::octet_stream();

	Ok(RegistryResponse::empty(
		HeadBlobResponseHeaders {
			content_type,
			docker_content_digest: DockerContentDigest::new(blob_record.digest),
			content_length: headers::ContentLength(blob_record.size as u64),
		},
		http::StatusCode::OK,
	))
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
