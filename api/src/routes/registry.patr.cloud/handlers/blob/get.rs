//! GET blob endpoint handler.
//!
//! This handler downloads a blob from the registry, streaming it directly from
//! S3. It supports HTTP range requests for partial downloads, which is useful
//! for resuming interrupted downloads or accessing specific parts of large
//! blobs.

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
		utils::{
			repository::verify_workspace_access,
			s3::{stream_from_s3, stream_from_s3_range},
		},
	},
};

/// Represents a parsed HTTP Range header
#[derive(Debug, Clone)]
struct RangeRequest {
	start: u64,
	end: Option<u64>,
}

/// Optional Range header wrapper that implements the Header trait
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OptionalRange(Option<headers::Range>);

impl OptionalRange {
	pub fn inner(&self) -> Option<&headers::Range> {
		self.0.as_ref()
	}
}

impl headers::Header for OptionalRange {
	fn name() -> &'static headers::HeaderName {
		headers::Range::name()
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		// Try to decode the Range header, but don't fail if it's missing
		match headers::Range::decode(values) {
			Ok(range) => Ok(Self(Some(range))),
			Err(_) => Ok(Self(None)),
		}
	}

	fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
		if let Some(ref range) = self.0 {
			range.encode(values);
		}
	}
}

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
	/// GET blob endpoint.
	///
	/// Downloads a blob from the registry, streaming it directly from S3.
	/// Supports HTTP range requests for partial downloads.
	GetBlob,
	GET "/v2/{name}/blobs/{digest}" {
		/// The repository name in the format workspace_id/repo_name
		pub name: String,
		/// The blob digest (sha256:...)
		pub digest: String,
	},
	auth = true,
	request_headers = {
		/// Optional Range header for partial downloads
		pub range: OptionalRange,
	},
	response_headers = {
		/// The content type of the blob
		pub content_type: headers::ContentType,
		/// The digest of the blob
		pub docker_content_digest: DockerContentDigest,
		/// The size of the blob in bytes (or range size)
		pub content_length: headers::ContentLength,
		/// Accept-Ranges header to indicate range support
		pub accept_ranges: headers::AcceptRanges,
	}
);

/// Handler for GET /v2/{name}/blobs/{digest}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Validates digest format
/// 4. Queries the database for blob metadata
/// 5. Streams blob content from S3
/// 6. Supports HTTP range requests for partial downloads
/// 7. Returns with appropriate headers
///
/// # Requirements
/// - 9.3: Stream blob content from S3 to client
/// - 10.1: Support range requests for partial downloads
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, GetBlobPath>,
) -> Result<RegistryResponse<GetBlobPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		digest = %req.path.digest,
		user_id = %req.user_data.id,
		"GET blob request"
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
		repo_name.workspace_id(),
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

	// 5. Use S3 bucket from request (pre-initialized in AppState)
	let bucket = req.s3_bucket;

	// 6. Construct S3 key for the blob
	let s3_key = format!("blobs/{}", blob_record.digest);

	// 7. Handle range requests
	let range_header = req.headers.range.inner();
	let total_size = blob_record.size as u64;

	let (body, content_length, status_code) = if let Some(range) = range_header {
		// Parse the range header
		// Range header format: "bytes=start-end" or "bytes=start-"
		match parse_range_header(range, total_size) {
			Ok((start, end)) => {
				info!(
					start = start,
					end = end,
					total_size = total_size,
					"Processing range request"
				);

				// Stream the requested range from S3
				let stream = stream_from_s3_range(&bucket, &s3_key, start, Some(end)).await?;

				let range_size = end - start + 1;

				(
					Body::from_stream(stream),
					range_size,
					http::StatusCode::PARTIAL_CONTENT,
				)
			}
			Err(e) => {
				warn!(
					error = %e,
					"Invalid range request"
				);
				return Err(RegistryError::unsupported(format!(
					"Invalid range request: {}",
					e
				)));
			}
		}
	} else {
		// No range request, stream the entire blob
		info!("Streaming entire blob");
		let stream = stream_from_s3(&bucket, &s3_key).await?;

		(Body::from_stream(stream), total_size, http::StatusCode::OK)
	};

	// 8. Return streaming response with appropriate headers
	info!(
		content_length = content_length,
		status = ?status_code,
		"Returning blob stream"
	);

	// ContentType for octet-stream
	let content_type = headers::ContentType::octet_stream();

	Ok(RegistryResponse::new(
		GetBlobResponseHeaders {
			content_type,
			docker_content_digest: DockerContentDigest::new(blob_record.digest),
			content_length: headers::ContentLength(content_length),
			accept_ranges: headers::AcceptRanges::bytes(),
		},
		body,
		status_code,
	))
}

/// Parse a Range header and return the start and end byte positions.
///
/// # Arguments
///
/// * `range` - The Range header value
/// * `total_size` - The total size of the blob
///
/// # Returns
///
/// A tuple of (start, end) byte positions (both inclusive)
///
/// # Errors
///
/// Returns an error if the range is invalid or unsatisfiable
fn parse_range_header(range: &headers::Range, total_size: u64) -> Result<(u64, u64), String> {
	use std::ops::Bound;

	// The headers::Range type can be satisfied against a total length
	// This returns an iterator of (Bound<u64>, Bound<u64>) tuples for satisfiable
	// ranges
	let mut satisfiable_ranges: Vec<_> = range.satisfiable_ranges(total_size).collect();

	if satisfiable_ranges.is_empty() {
		return Err("No satisfiable ranges".to_string());
	}

	// We only support single range requests for now
	if satisfiable_ranges.len() > 1 {
		return Err("Multiple ranges not supported".to_string());
	}

	let (start_bound, end_bound) = satisfiable_ranges.remove(0);

	// Convert bounds to concrete values
	let start = match start_bound {
		Bound::Included(n) => n,
		Bound::Excluded(n) => n + 1,
		Bound::Unbounded => 0,
	};

	let end = match end_bound {
		Bound::Included(n) => n,
		Bound::Excluded(n) => n.saturating_sub(1),
		Bound::Unbounded => total_size.saturating_sub(1),
	};

	Ok((start, end))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_blob_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<GetBlobPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/{digest}"
		);
	}

	#[test]
	fn test_parse_range_header_full_range() {
		// Test parsing a full range specification
		let total_size = 1000u64;

		// Create a range for bytes 0-499
		let range = headers::Range::bytes(0..500).unwrap();
		let (start, end) = parse_range_header(&range, total_size).unwrap();

		assert_eq!(start, 0);
		assert_eq!(end, 499);
	}

	#[test]
	fn test_parse_range_header_open_ended() {
		// Test parsing an open-ended range (from start to end of file)
		let total_size = 1000u64;

		// Create a range for bytes 500-
		let range = headers::Range::bytes(500..).unwrap();
		let (start, end) = parse_range_header(&range, total_size).unwrap();

		assert_eq!(start, 500);
		assert_eq!(end, 999); // Should be clamped to total_size - 1
	}

	#[test]
	fn test_parse_range_header_exceeds_size() {
		// Test that ranges exceeding the file size are handled correctly
		let total_size = 1000u64;

		// Create a range that exceeds the file size
		let range = headers::Range::bytes(500..2000).unwrap();
		let (start, end) = parse_range_header(&range, total_size).unwrap();

		assert_eq!(start, 500);
		assert_eq!(end, 999); // Should be clamped to total_size - 1
	}

	#[test]
	fn test_parse_range_header_invalid_start() {
		// Test that a range starting beyond the file size is rejected
		let total_size = 1000u64;

		let range = headers::Range::bytes(1500..).unwrap();
		let result = parse_range_header(&range, total_size);

		assert!(result.is_err());
	}
}
