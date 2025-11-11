//! GET blob upload status endpoint handler.
//!
//! This handler retrieves the status of an ongoing blob upload session,
//! returning the current byte range that has been uploaded.

use axum::body::Body;
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
		Self(format!("{}-{}", start, end))
	}

	pub fn from_last_byte(last_byte: i64) -> Self {
		if last_byte == 0 {
			Self("0-0".to_string())
		} else {
			Self(format!("0-{}", last_byte))
		}
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
	/// GET blob upload status endpoint.
	///
	/// Retrieves the status of an ongoing blob upload session.
	GetBlobUploadStatus,
	GET "/v2/{name}/blobs/uploads/{uuid}" {
		/// The repository name in the format workspace_id/repo_name
		pub name: String,
		/// The upload session UUID
		pub uuid: String,
	},
	auth = true,
	response_headers = {
		/// The current byte range that has been uploaded
		pub range: RangeHeader,
		/// The UUID for this upload session
		pub docker_upload_uuid: DockerUploadUuid,
	}
);

/// Handler for GET /v2/{name}/blobs/uploads/{uuid}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Retrieves upload session from database
/// 4. Returns 204 No Content with Range and Docker-Upload-UUID headers
///
/// # Requirements
/// - 6.3: Query upload status and return current byte range
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, GetBlobUploadStatusPath>,
) -> Result<RegistryResponse<GetBlobUploadStatusPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		uuid = %req.path.uuid,
		user_id = %req.user_data.id,
		"GET blob upload status request"
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

	// 3. Parse UUID
	let session_id = Uuid::parse_str(&req.path.uuid)
		.map_err(|_| RegistryError::blob_upload_unknown(&req.path.uuid))?;

	// 4. Retrieve upload session from database
	#[derive(Debug)]
	struct SessionRecord {
		last_byte: i64,
	}

	let session = sqlx::query_as!(
		SessionRecord,
		r#"
		SELECT last_byte
		FROM container_registry_session
		WHERE id = $1
			AND user_id = $2
		"#,
		session_id as _,
		req.user_data.id as _
	)
	.fetch_optional(&mut **req.database)
	.await?
	.ok_or_else(|| {
		warn!(
			session_id = %session_id,
			user_id = %req.user_data.id,
			"Upload session not found"
		);
		RegistryError::blob_upload_unknown(&req.path.uuid)
	})?;

	info!(
		session_id = %session_id,
		last_byte = session.last_byte,
		"Retrieved upload session status"
	);

	// 5. Return 204 No Content with Range and Docker-Upload-UUID headers
	Ok(RegistryResponse::new(
		GetBlobUploadStatusResponseHeaders {
			range: RangeHeader::from_last_byte(session.last_byte),
			docker_upload_uuid: DockerUploadUuid::new(req.path.uuid.clone()),
		},
		Body::empty(),
		http::StatusCode::NO_CONTENT,
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_blob_upload_status_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<GetBlobUploadStatusPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/uploads/{uuid}"
		);
	}

	#[test]
	fn test_docker_upload_uuid_header() {
		let uuid = DockerUploadUuid::new("550e8400-e29b-41d4-a716-446655440000".to_string());
		assert_eq!(uuid.0, "550e8400-e29b-41d4-a716-446655440000");
	}

	#[test]
	fn test_range_header_from_last_byte_zero() {
		let range = RangeHeader::from_last_byte(0);
		assert_eq!(range.0, "0-0");
	}

	#[test]
	fn test_range_header_from_last_byte() {
		let range = RangeHeader::from_last_byte(1023);
		assert_eq!(range.0, "0-1023");
	}

	#[test]
	fn test_range_header_new() {
		let range = RangeHeader::new(0, 1023);
		assert_eq!(range.0, "0-1023");
	}
}
