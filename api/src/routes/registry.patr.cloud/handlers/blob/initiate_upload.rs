//! POST blob upload initiation endpoint handler.
//!
//! This handler initiates a blob upload session, which allows clients to upload
//! large blobs in chunks using the chunked upload protocol. It also supports
//! cross-repository blob mounting for efficient blob sharing.

use axum::body::Body;
use http::HeaderValue;

use super::mount::{get_or_create_repository, handle_blob_mount};
use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		AuthenticatedRegistryRequest,
		RegistryEndpoint,
		RegistryError,
		RegistryResponse,
		types::RepositoryName,
		utils::{repository::verify_workspace_access, s3::initiate_multipart_upload},
	},
};

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

/// Custom header for Docker upload UUID
#[derive(Debug, Clone, PartialEq)]
pub struct DockerUploadUuid(String);

impl DockerUploadUuid {
	pub fn new(uuid: String) -> Self {
		Self(uuid)
	}
}

impl headers::Header for DockerUploadUuid {
	fn name() -> &'static headers::HeaderName {
		static NAME: headers::HeaderName = headers::HeaderName::from_static("docker-upload-uuid");
		&NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;
		let uuid = value
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.to_string();
		Ok(Self(uuid))
	}

	fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
		if let Ok(value) = HeaderValue::from_str(&self.0) {
			values.extend(std::iter::once(value));
		}
	}
}

/// Custom header for Range (used in upload responses)
#[derive(Debug, Clone, PartialEq)]
pub struct RangeHeader(String);

impl RangeHeader {
	pub fn new(start: u64, end: u64) -> Self {
		Self(format!("0-{}", end))
	}

	pub fn zero() -> Self {
		Self("0-0".to_string())
	}
}

impl headers::Header for RangeHeader {
	fn name() -> &'static headers::HeaderName {
		static NAME: headers::HeaderName = headers::HeaderName::from_static("range");
		&NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;
		let range = value
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.to_string();
		Ok(Self(range))
	}

	fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
		if let Ok(value) = HeaderValue::from_str(&self.0) {
			values.extend(std::iter::once(value));
		}
	}
}

macros::declare_registry_endpoint!(
	/// POST blob upload initiation endpoint.
	///
	/// Initiates a blob upload session for chunked uploads or handles
	/// cross-repository blob mounting.
	InitiateBlobUpload,
	POST "/v2/{name}/blobs/uploads/" {
		/// The repository name in the format workspace_id/repo_name
		pub name: String,
	},
	query = {
		/// Optional digest for cross-repository blob mounting
		pub mount: Option<String>,
		/// Optional source repository for blob mounting
		pub from: Option<String>,
	},
	response_headers = {
		/// Location header pointing to the upload URL
		pub location: Location,
		/// The UUID for this upload session
		pub docker_upload_uuid: DockerUploadUuid,
		/// The current byte range (0-0 for new uploads)
		pub range: RangeHeader,
	}
);

/// Handler for POST /v2/{name}/blobs/uploads/
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Checks for mount query parameters (mount and from)
/// 4. If mount requested, handles cross-repository blob mounting
/// 5. Otherwise, creates new upload session with UUID
/// 6. Initiates S3 multipart upload
/// 7. Stores session in database with S3 upload ID
/// 8. Returns 202 Accepted with Location, Docker-Upload-UUID, and Range headers
///
/// # Requirements
/// - 6.1: Stream data directly to S3 without buffering
/// - 6.3: Create S3 multipart upload session
/// - 6.7: Support cross-repository blob mounting
/// - 9.5: Support blob mounting from another repository
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, InitiateBlobUploadPath>,
) -> Result<RegistryResponse<InitiateBlobUploadPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		user_id = %req.user_data.id,
		mount = ?req.query.mount,
		from = ?req.query.from,
		"POST blob upload initiation request"
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

	// 3. Check for mount query parameters
	if let (Some(mount_digest), Some(from_repo)) = (&req.query.mount, &req.query.from) {
		info!(
			mount_digest = %mount_digest,
			from_repo = %from_repo,
			"Attempting cross-repository blob mount"
		);

		// Handle cross-repository blob mounting
		// If mount succeeds, return 201 Created
		// If mount fails, fall through to create new upload session
		match handle_blob_mount(&mut req, &repo_name, mount_digest, from_repo).await {
			Ok(response) => return Ok(response),
			Err(e) => {
				warn!(
					error = %e,
					"Blob mount failed, falling back to new upload session"
				);
				// Fall through to create new upload session
			}
		}
	}

	// 4. Get or create repository in database
	let repository_id =
		get_or_create_repository(req.database, repo_name.workspace_id(), repo_name.name()).await?;

	info!(
		repository_id = %repository_id,
		"Repository found or created"
	);

	// 5. Create new upload session with UUID
	let session_id = Uuid::new_v4();
	debug!(
		session_id = %session_id,
		"Generated new upload session ID"
	);

	// 6. Initiate S3 multipart upload
	let bucket = req.s3_bucket;
	let s3_key = format!("uploads/{}", session_id);

	let upload_id = initiate_multipart_upload(&bucket, &s3_key).await?;
	info!(
		s3_key = %s3_key,
		upload_id = %upload_id,
		"Initiated S3 multipart upload"
	);

	// 7. Store session in database
	sqlx::query!(
		r#"
		INSERT INTO container_registry_session (
			id,
			user_id,
			aws_session_id,
			blob_digest,
			current_part,
			last_byte,
			parts,
			updated_at
		) VALUES ($1, $2, $3, NULL, 0, 0, ARRAY[]::container_registry_session_parts[], NOW())
		"#,
		session_id as _,
		req.user_data.id as _,
		upload_id,
	)
	.execute(&mut **req.database)
	.await?;

	info!(
		session_id = %session_id,
		"Created upload session in database"
	);

	// 8. Build Location header
	let location_url = format!("/v2/{}/blobs/uploads/{}", req.path.name, session_id);

	// 9. Return 202 Accepted with headers
	info!(
		session_id = %session_id,
		location = %location_url,
		"Returning upload session details"
	);

	Ok(RegistryResponse::new(
		InitiateBlobUploadResponseHeaders {
			location: Location::new(location_url),
			docker_upload_uuid: DockerUploadUuid::new(session_id.to_string()),
			range: RangeHeader::zero(),
		},
		Body::empty(),
		http::StatusCode::ACCEPTED,
	))
}

/// Create an S3 Bucket client
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_initiate_blob_upload_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<InitiateBlobUploadPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/uploads/"
		);
	}

	#[test]
	fn test_docker_upload_uuid_header() {
		let uuid = DockerUploadUuid::new("550e8400-e29b-41d4-a716-446655440000".to_string());
		assert_eq!(uuid.0, "550e8400-e29b-41d4-a716-446655440000");
	}

	#[test]
	fn test_range_header_zero() {
		let range = RangeHeader::zero();
		assert_eq!(range.0, "0-0");
	}

	#[test]
	fn test_range_header_new() {
		let range = RangeHeader::new(0, 1023);
		assert_eq!(range.0, "0-1023");
	}
}
