/// Utility functions for registry operations.
///
/// This module contains utilities for:
/// - S3 streaming operations (upload/download without buffering)
/// - Repository access validation
/// - Digest verification
/// - Tag resolution
/// - Blob reference checking
/// - Background cleanup tasks

/// Blob reference checking utilities for safe deletion operations.
pub mod blob;
/// Background cleanup tasks for expired upload sessions.
pub mod cleanup;
/// Digest computation and verification utilities.
pub mod digest;
pub mod repository;
/// S3 streaming utilities for blob storage operations.
pub mod s3;
/// Tag resolution utilities for converting tag names to manifest digests.
pub mod tag;
