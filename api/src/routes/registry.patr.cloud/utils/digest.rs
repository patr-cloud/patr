//! Digest verification utilities.
//!
//! This module provides utilities for computing and verifying SHA256 digests
//! of content streams, ensuring content integrity in the OCI registry.

use sha2::{Digest, Sha256};
use tokio::io::AsyncRead;

use crate::routes::registry_patr_cloud::prelude::RegistryError;

/// Compute SHA256 digest from a byte slice.
///
/// # Arguments
///
/// * `data` - The data to compute the digest for
///
/// # Returns
///
/// Returns the digest in the format "sha256:hex_string"
///
/// # Example
///
/// ```ignore
/// let data = b"hello world";
/// let digest = compute_digest_from_bytes(data);
/// assert_eq!(digest, "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
/// ```
pub fn compute_digest_from_bytes(data: &[u8]) -> String {
	let mut hasher = Sha256::new();
	hasher.update(data);
	format!("sha256:{:x}", hasher.finalize())
}

/// Compute SHA256 digest from an async reader stream.
///
/// This function reads the entire stream and computes its SHA256 digest
/// without buffering the entire content in memory at once.
///
/// # Arguments
///
/// * `reader` - An async reader to compute the digest for
///
/// # Returns
///
/// Returns a tuple of (digest, total_bytes_read) where digest is in the
/// format "sha256:hex_string"
///
/// # Errors
///
/// Returns `RegistryError` if reading from the stream fails
pub async fn compute_digest_from_stream<R>(mut reader: R) -> Result<(String, u64), RegistryError>
where
	R: AsyncRead + Unpin,
{
	use tokio::io::AsyncReadExt;

	let mut hasher = Sha256::new();
	let mut buffer = vec![0u8; 8192]; // 8KB buffer
	let mut total_bytes = 0u64;

	loop {
		let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
			tracing::error!(error = %e, "Failed to read from stream for digest computation");
			RegistryError::from(e)
		})?;

		if bytes_read == 0 {
			break;
		}

		hasher.update(&buffer[..bytes_read]);
		total_bytes += bytes_read as u64;
	}

	let digest = format!("sha256:{:x}", hasher.finalize());
	Ok((digest, total_bytes))
}

/// Verify that a computed digest matches an expected digest.
///
/// # Arguments
///
/// * `computed` - The computed digest
/// * `expected` - The expected digest
///
/// # Returns
///
/// Returns `Ok(())` if digests match, otherwise returns a `RegistryError`
/// with DIGEST_INVALID error code
///
/// # Example
///
/// ```ignore
/// let computed = "sha256:abc123...";
/// let expected = "sha256:abc123...";
/// verify_digest_match(computed, expected)?;
/// ```
pub fn verify_digest_match(computed: &str, expected: &str) -> Result<(), RegistryError> {
	if computed != expected {
		tracing::error!(
			computed = %computed,
			expected = %expected,
			"Digest mismatch detected"
		);
		return Err(RegistryError::digest_invalid(format!(
			"Digest mismatch: expected {}, got {}",
			expected, computed
		)));
	}

	tracing::debug!(
		digest = %computed,
		"Digest verification successful"
	);

	Ok(())
}

/// Compute digest from bytes and verify it matches expected digest.
///
/// This is a convenience function that combines digest computation and
/// verification in a single call.
///
/// # Arguments
///
/// * `data` - The data to compute the digest for
/// * `expected_digest` - The expected digest to verify against
///
/// # Returns
///
/// Returns `Ok(())` if the computed digest matches the expected digest
///
/// # Errors
///
/// Returns `RegistryError` with DIGEST_INVALID if digests don't match
pub fn compute_and_verify_digest(data: &[u8], expected_digest: &str) -> Result<(), RegistryError> {
	let computed_digest = compute_digest_from_bytes(data);
	verify_digest_match(&computed_digest, expected_digest)
}

/// Compute digest from stream and verify it matches expected digest.
///
/// This function reads the entire stream, computes its SHA256 digest,
/// and verifies it matches the expected digest.
///
/// # Arguments
///
/// * `reader` - An async reader to compute the digest for
/// * `expected_digest` - The expected digest to verify against
///
/// # Returns
///
/// Returns `Ok(total_bytes)` if the computed digest matches the expected
/// digest, where total_bytes is the number of bytes read from the stream
///
/// # Errors
///
/// Returns `RegistryError` if:
/// - Reading from the stream fails
/// - The computed digest doesn't match the expected digest
pub async fn compute_and_verify_digest_from_stream<R>(
	reader: R,
	expected_digest: &str,
) -> Result<u64, RegistryError>
where
	R: AsyncRead + Unpin,
{
	let (computed_digest, total_bytes) = compute_digest_from_stream(reader).await?;
	verify_digest_match(&computed_digest, expected_digest)?;
	Ok(total_bytes)
}

/// Validate digest format.
///
/// Ensures the digest is in the correct format: "sha256:" followed by
/// 64 hexadecimal characters.
///
/// # Arguments
///
/// * `digest` - The digest string to validate
///
/// # Returns
///
/// Returns `Ok(())` if the digest format is valid
///
/// # Errors
///
/// Returns `RegistryError` with DIGEST_INVALID if the format is invalid
pub fn validate_digest_format(digest: &str) -> Result<(), RegistryError> {
	if !digest.starts_with("sha256:") {
		return Err(RegistryError::digest_invalid(format!(
			"Digest must start with 'sha256:', got: {}",
			digest
		)));
	}

	let hash_part = &digest[7..]; // Skip "sha256:"
	if hash_part.len() != 64 {
		return Err(RegistryError::digest_invalid(format!(
			"SHA256 digest must be 64 hex characters, got {} characters",
			hash_part.len()
		)));
	}

	if !hash_part.chars().all(|c| c.is_ascii_hexdigit()) {
		return Err(RegistryError::digest_invalid(format!(
			"Digest contains non-hexadecimal characters: {}",
			digest
		)));
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::io::Cursor;

	use super::*;

	#[test]
	fn test_compute_digest_from_bytes() {
		let data = b"hello world";
		let digest = compute_digest_from_bytes(data);
		assert_eq!(
			digest,
			"sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
		);
	}

	#[test]
	fn test_compute_digest_empty() {
		let data = b"";
		let digest = compute_digest_from_bytes(data);
		assert_eq!(
			digest,
			"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
		);
	}

	#[tokio::test]
	async fn test_compute_digest_from_stream() {
		let data = b"hello world";
		let cursor = Cursor::new(data);
		let (digest, size) = compute_digest_from_stream(cursor).await.unwrap();
		assert_eq!(
			digest,
			"sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
		);
		assert_eq!(size, 11);
	}

	#[tokio::test]
	async fn test_compute_digest_from_stream_large() {
		// Test with data larger than buffer size (8KB)
		let data = vec![b'a'; 16384]; // 16KB
		let cursor = Cursor::new(data);
		let (digest, size) = compute_digest_from_stream(cursor).await.unwrap();
		assert_eq!(size, 16384);
		assert!(digest.starts_with("sha256:"));
		assert_eq!(digest.len(), 71); // "sha256:" + 64 hex chars
	}

	#[test]
	fn test_verify_digest_match_success() {
		let digest = "sha256:abc123";
		let result = verify_digest_match(digest, digest);
		assert!(result.is_ok());
	}

	#[test]
	fn test_verify_digest_match_failure() {
		let computed = "sha256:abc123";
		let expected = "sha256:def456";
		let result = verify_digest_match(computed, expected);
		assert!(result.is_err());
	}

	#[test]
	fn test_compute_and_verify_digest_success() {
		let data = b"hello world";
		let expected = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
		let result = compute_and_verify_digest(data, expected);
		assert!(result.is_ok());
	}

	#[test]
	fn test_compute_and_verify_digest_failure() {
		let data = b"hello world";
		let expected = "sha256:wrong_digest_here_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
		let result = compute_and_verify_digest(data, expected);
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_compute_and_verify_digest_from_stream_success() {
		let data = b"hello world";
		let cursor = Cursor::new(data);
		let expected = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
		let result = compute_and_verify_digest_from_stream(cursor, expected).await;
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), 11);
	}

	#[tokio::test]
	async fn test_compute_and_verify_digest_from_stream_failure() {
		let data = b"hello world";
		let cursor = Cursor::new(data);
		let expected = "sha256:wrong_digest_here_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
		let result = compute_and_verify_digest_from_stream(cursor, expected).await;
		assert!(result.is_err());
	}

	#[test]
	fn test_validate_digest_format_valid() {
		let digest = "sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
		let result = validate_digest_format(digest);
		assert!(result.is_ok());
	}

	#[test]
	fn test_validate_digest_format_missing_prefix() {
		let digest = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
		let result = validate_digest_format(digest);
		assert!(result.is_err());
	}

	#[test]
	fn test_validate_digest_format_wrong_length() {
		let digest = "sha256:abc123"; // Too short
		let result = validate_digest_format(digest);
		assert!(result.is_err());
	}

	#[test]
	fn test_validate_digest_format_non_hex() {
		let digest = "sha256:g94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"; // 'g' is not hex
		let result = validate_digest_format(digest);
		assert!(result.is_err());
	}

	#[test]
	fn test_validate_digest_format_uppercase() {
		let digest = "sha256:B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9";
		let result = validate_digest_format(digest);
		assert!(result.is_ok()); // Uppercase hex is valid
	}
}
