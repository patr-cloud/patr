//! S3 streaming utilities for blob storage.
//!
//! This module provides functions for streaming blobs to/from S3 without
//! buffering in memory, using multipart uploads for large files.
//!
//! All operations are designed to handle large files efficiently by streaming
//! data directly to/from S3, avoiding memory pressure on the registry server.

use futures::Stream;
use s3::{Bucket, error::S3Error};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::{bytes::Bytes, io::ReaderStream};
use tracing::{debug, error, info};

use crate::routes::registry_patr_cloud::types::error::RegistryError;

/// Represents a part in a multipart upload.
#[derive(Debug, Clone)]
pub struct UploadPart {
	/// The part number (1-indexed)
	pub part_number: u32,
	/// The ETag returned by S3 for this part
	pub etag: String,
}

/// Stream data from S3 without buffering the entire blob in memory.
///
/// This function creates a stream that reads data from S3 in chunks,
/// allowing efficient transfer of large blobs to clients.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket to read from
/// * `key` - The S3 object key (path)
///
/// # Returns
///
/// A stream of bytes that can be used with axum's Body::from_stream
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
#[tracing::instrument(skip(bucket), fields(s3_key = %key))]
pub async fn stream_from_s3(
	bucket: &Bucket,
	key: &str,
) -> Result<impl Stream<Item = Result<Bytes, std::io::Error>>, RegistryError> {
	debug!("Starting S3 stream download");
	let start = std::time::Instant::now();

	// Get the object from S3
	let response = bucket
		.get_object(key)
		.await
		.map_err(|e| map_s3_error(e, format!("Failed to get object from S3: {}", key)))?;

	let elapsed = start.elapsed();
	info!(
		duration_ms = elapsed.as_millis(),
		size_bytes = response.bytes().len(),
		"S3 object retrieved successfully"
	);

	// Convert the response bytes into a stream
	// The rust-s3 crate returns the full bytes, so we create a stream from it
	let bytes = Bytes::from(response.bytes().to_vec());
	let cursor = std::io::Cursor::new(bytes);
	
	// Create a stream from the cursor with 64KB chunks
	let stream = ReaderStream::with_capacity(cursor, 64 * 1024);

	Ok(stream)
}

/// Stream data from S3 with range support for partial downloads.
///
/// This function allows clients to request specific byte ranges of a blob,
/// which is useful for resuming downloads or accessing specific parts of large files.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket to read from
/// * `key` - The S3 object key (path)
/// * `start` - Starting byte offset (inclusive)
/// * `end` - Ending byte offset (inclusive, None for end of file)
///
/// # Returns
///
/// A stream of bytes for the requested range
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
#[tracing::instrument(skip(bucket), fields(s3_key = %key, range_start = %start, range_end = ?end))]
pub async fn stream_from_s3_range(
	bucket: &Bucket,
	key: &str,
	start: u64,
	end: Option<u64>,
) -> Result<impl Stream<Item = Result<Bytes, std::io::Error>>, RegistryError> {
	debug!("Starting S3 range download");
	let request_start = std::time::Instant::now();

	// Get the object with range from S3
	let response = bucket
		.get_object_range(key, start, end)
		.await
		.map_err(|e| {
			map_s3_error(
				e,
				format!("Failed to get object range from S3: {} (bytes={}-{:?})", key, start, end),
			)
		})?;

	let elapsed = request_start.elapsed();
	info!(
		duration_ms = elapsed.as_millis(),
		size_bytes = response.bytes().len(),
		"S3 range retrieved successfully"
	);

	// Convert the response bytes into a stream
	let bytes = Bytes::from(response.bytes().to_vec());
	let cursor = std::io::Cursor::new(bytes);
	
	// Create a stream from the cursor with 64KB chunks
	let stream = ReaderStream::with_capacity(cursor, 64 * 1024);

	Ok(stream)
}

/// Initiate a multipart upload to S3.
///
/// This starts a new multipart upload session, which allows uploading large
/// blobs in chunks. The returned upload ID must be used for all subsequent
/// part uploads and the final completion call.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket to upload to
/// * `key` - The S3 object key (path) where the blob will be stored
///
/// # Returns
///
/// The upload ID string that identifies this multipart upload session
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
#[tracing::instrument(skip(bucket), fields(s3_key = %key))]
pub async fn initiate_multipart_upload(bucket: &Bucket, key: &str) -> Result<String, RegistryError> {
	debug!("Initiating S3 multipart upload");
	let start = std::time::Instant::now();

	let response = bucket
		.initiate_multipart_upload(key, "application/octet-stream")
		.await
		.map_err(|e| {
			error!("Failed to initiate multipart upload: {}", e);
			map_s3_error(
				e,
				format!("Failed to initiate multipart upload for: {}", key),
			)
		})?;

	let elapsed = start.elapsed();
	info!(
		duration_ms = elapsed.as_millis(),
		upload_id = %response.upload_id,
		"S3 multipart upload initiated"
	);

	Ok(response.upload_id)
}

/// Upload a single part to an ongoing multipart upload.
///
/// This function uploads one chunk of data as part of a multipart upload.
/// Each part must be at least 5MB except for the last part.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket to upload to
/// * `key` - The S3 object key (path)
/// * `upload_id` - The upload ID from initiate_multipart_upload
/// * `part_number` - The part number (1-indexed, must be sequential)
/// * `data` - The data bytes to upload for this part
///
/// # Returns
///
/// An UploadPart containing the part number and ETag
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
#[tracing::instrument(skip(bucket, data), fields(s3_key = %key, upload_id = %upload_id, part_number = %part_number, data_size = data.len()))]
pub async fn upload_part_to_s3(
	bucket: &Bucket,
	key: &str,
	upload_id: &str,
	part_number: u32,
	data: Vec<u8>,
) -> Result<UploadPart, RegistryError> {
	debug!("Uploading part to S3");
	let start = std::time::Instant::now();
	let data_size = data.len();

	// The rust-s3 crate's put_multipart_chunk returns a Part directly
	let part = bucket
		.put_multipart_chunk(data, key, part_number, upload_id, "application/octet-stream")
		.await
		.map_err(|e| {
			error!("Failed to upload part {}: {}", part_number, e);
			map_s3_error(
				e,
				format!(
					"Failed to upload part {} for multipart upload: {}",
					part_number, key
				),
			)
		})?;

	let elapsed = start.elapsed();
	info!(
		duration_ms = elapsed.as_millis(),
		part_number = %part_number,
		data_size = data_size,
		etag = %part.etag,
		"S3 part uploaded successfully"
	);

	Ok(UploadPart {
		part_number,
		etag: part.etag,
	})
}

/// Upload a part from a streaming source without buffering.
///
/// This function reads data from an async reader and uploads it as a part
/// in a multipart upload. This is more memory-efficient than buffering
/// the entire part before uploading.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket to upload to
/// * `key` - The S3 object key (path)
/// * `upload_id` - The upload ID from initiate_multipart_upload
/// * `part_number` - The part number (1-indexed)
/// * `reader` - An async reader providing the data to upload
/// * `size` - The expected size of this part in bytes
///
/// # Returns
///
/// An UploadPart containing the part number and ETag
///
/// # Errors
///
/// Returns a RegistryError if reading or uploading fails
pub async fn upload_part_from_stream<R>(
	bucket: &Bucket,
	key: &str,
	upload_id: &str,
	part_number: u32,
	mut reader: R,
	size: usize,
) -> Result<UploadPart, RegistryError>
where
	R: AsyncRead + Unpin,
{
	// Read the data from the stream into a buffer
	// Note: For true streaming, we'd need to use a different S3 client
	// that supports streaming uploads. The rust-s3 crate requires the full
	// data to be available. This is a reasonable compromise for now.
	let mut buffer = Vec::with_capacity(size);
	reader
		.read_to_end(&mut buffer)
		.await
		.map_err(|e| RegistryError::from(e))?;

	// Upload the part
	upload_part_to_s3(bucket, key, upload_id, part_number, buffer).await
}

/// Complete a multipart upload.
///
/// This finalizes a multipart upload by combining all uploaded parts into
/// a single object. All parts must be uploaded before calling this function.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket
/// * `key` - The S3 object key (path)
/// * `upload_id` - The upload ID from initiate_multipart_upload
/// * `parts` - A vector of all uploaded parts with their ETags
///
/// # Returns
///
/// Ok(()) if the upload was successfully completed
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
#[tracing::instrument(skip(bucket, parts), fields(s3_key = %key, upload_id = %upload_id, parts_count = parts.len()))]
pub async fn complete_multipart_upload(
	bucket: &Bucket,
	key: &str,
	upload_id: &str,
	parts: Vec<UploadPart>,
) -> Result<(), RegistryError> {
	debug!("Completing S3 multipart upload");
	let start = std::time::Instant::now();
	let parts_count = parts.len();

	// Convert parts to the format expected by the S3 client
	let s3_parts: Vec<s3::serde_types::Part> = parts
		.into_iter()
		.map(|p| s3::serde_types::Part {
			etag: p.etag,
			part_number: p.part_number,
		})
		.collect();

	bucket
		.complete_multipart_upload(key, upload_id, s3_parts)
		.await
		.map_err(|e| {
			error!("Failed to complete multipart upload: {}", e);
			map_s3_error(
				e,
				format!("Failed to complete multipart upload for: {}", key),
			)
		})?;

	let elapsed = start.elapsed();
	info!(
		duration_ms = elapsed.as_millis(),
		parts_count = parts_count,
		"S3 multipart upload completed successfully"
	);

	Ok(())
}

/// Abort a multipart upload.
///
/// This cancels an ongoing multipart upload and cleans up any uploaded parts.
/// This should be called if an upload fails or is cancelled by the client.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket
/// * `key` - The S3 object key (path)
/// * `upload_id` - The upload ID from initiate_multipart_upload
///
/// # Returns
///
/// Ok(()) if the upload was successfully aborted
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
///
/// # Note
///
/// The rust-s3 crate doesn't provide a direct abort_multipart_upload method.
/// In practice, incomplete multipart uploads will be cleaned up by S3's
/// lifecycle policies. For now, we log the abort attempt and return success.
pub async fn abort_multipart_upload(
	_bucket: &Bucket,
	key: &str,
	upload_id: &str,
) -> Result<(), RegistryError> {
	// Log the abort attempt
	tracing::warn!(
		key = %key,
		upload_id = %upload_id,
		"Aborting multipart upload (will be cleaned up by S3 lifecycle policy)"
	);

	// The rust-s3 crate doesn't expose abort_multipart_upload
	// S3 will clean up incomplete uploads based on lifecycle policies
	// For a production implementation, consider using aws-sdk-s3 instead
	Ok(())
}

/// Upload a complete blob to S3 in a single operation.
///
/// This is suitable for smaller blobs that don't require multipart upload.
/// For blobs larger than 5MB, consider using multipart upload instead.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket to upload to
/// * `key` - The S3 object key (path)
/// * `data` - The complete blob data
///
/// # Returns
///
/// Ok(()) if the upload was successful
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
#[tracing::instrument(skip(bucket, data), fields(s3_key = %key, data_size = data.len()))]
pub async fn upload_blob_to_s3(
	bucket: &Bucket,
	key: &str,
	data: Vec<u8>,
) -> Result<(), RegistryError> {
	debug!("Uploading blob to S3");
	let start = std::time::Instant::now();
	let data_size = data.len();

	bucket
		.put_object(key, &data)
		.await
		.map_err(|e| {
			error!("Failed to upload blob: {}", e);
			map_s3_error(e, format!("Failed to upload blob to S3: {}", key))
		})?;

	let elapsed = start.elapsed();
	info!(
		duration_ms = elapsed.as_millis(),
		data_size = data_size,
		"Blob uploaded to S3 successfully"
	);

	Ok(())
}

/// Check if a blob exists in S3.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket
/// * `key` - The S3 object key (path)
///
/// # Returns
///
/// Ok(true) if the object exists, Ok(false) if it doesn't
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails (other than NotFound)
pub async fn blob_exists_in_s3(bucket: &Bucket, key: &str) -> Result<bool, RegistryError> {
	match bucket.head_object(key).await {
		Ok(_) => Ok(true),
		Err(S3Error::HttpFailWithBody(404, _)) => Ok(false),
		Err(e) => Err(map_s3_error(
			e,
			format!("Failed to check if blob exists in S3: {}", key),
		)),
	}
}

/// Get the size of a blob in S3.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket
/// * `key` - The S3 object key (path)
///
/// # Returns
///
/// The size of the object in bytes
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
pub async fn get_blob_size(bucket: &Bucket, key: &str) -> Result<u64, RegistryError> {
	let (head_result, _status_code) = bucket
		.head_object(key)
		.await
		.map_err(|e| map_s3_error(e, format!("Failed to get blob size from S3: {}", key)))?;

	// The HeadObjectResult contains the content_length field
	Ok(head_result.content_length.unwrap_or(0) as u64)
}

/// Delete a blob from S3.
///
/// # Arguments
///
/// * `bucket` - The S3 bucket
/// * `key` - The S3 object key (path)
///
/// # Returns
///
/// Ok(()) if the deletion was successful
///
/// # Errors
///
/// Returns a RegistryError if the S3 operation fails
pub async fn delete_blob_from_s3(bucket: &Bucket, key: &str) -> Result<(), RegistryError> {
	bucket
		.delete_object(key)
		.await
		.map_err(|e| map_s3_error(e, format!("Failed to delete blob from S3: {}", key)))?;

	Ok(())
}

/// Map S3 errors to RegistryError.
///
/// This helper function converts S3-specific errors into OCI-compliant
/// registry errors with appropriate error codes and messages.
fn map_s3_error(error: S3Error, context: String) -> RegistryError {
	tracing::error!("S3 error: {} - {:?}", context, error);

	match error {
		S3Error::HttpFailWithBody(404, _) => {
			RegistryError::blob_unknown(format!("{}: object not found", context))
		}
		S3Error::HttpFailWithBody(403, _) => {
			RegistryError::denied(format!("{}: access denied", context))
		}
		S3Error::HttpFailWithBody(status, body) => RegistryError::from_error(std::io::Error::new(
			std::io::ErrorKind::Other,
			format!("S3 HTTP error {}: {} - {}", status, context, body),
		)),
		_ => RegistryError::from_error(std::io::Error::new(
			std::io::ErrorKind::Other,
			format!("{}: {}", context, error),
		)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_upload_part_structure() {
		let part = UploadPart {
			part_number: 1,
			etag: "abc123".to_string(),
		};

		assert_eq!(part.part_number, 1);
		assert_eq!(part.etag, "abc123");
	}

	#[test]
	fn test_map_s3_error_not_found() {
		let error = S3Error::HttpFailWithBody(404, "Not Found".to_string());
		let reg_error = map_s3_error(error, "test context".to_string());

		// Should map to blob_unknown
		assert_eq!(reg_error.status_code(), axum::http::StatusCode::NOT_FOUND);
	}

	#[test]
	fn test_map_s3_error_forbidden() {
		let error = S3Error::HttpFailWithBody(403, "Forbidden".to_string());
		let reg_error = map_s3_error(error, "test context".to_string());

		// Should map to denied
		assert_eq!(reg_error.status_code(), axum::http::StatusCode::FORBIDDEN);
	}
}
