/// Blob reference checking utilities.
///
/// This module provides functions for checking if blobs are referenced
/// by manifests, which is essential for safe deletion operations.
///
/// # Usage Example
///
/// ```ignore
/// use crate::routes::registry_patr_cloud::utils::blob::is_blob_referenced;
///
/// // In a handler function:
/// pub async fn handler(
///     req: AuthenticatedRegistryRequest<'_, DeleteBlob>
/// ) -> Result<RegistryResponse<DeleteBlob>, RegistryError> {
///     let digest = &req.path.digest;
///     
///     // Check if blob is referenced before deleting
///     if is_blob_referenced(req.database, digest).await? {
///         return Err(RegistryError::denied(
///             "Cannot delete blob: still referenced by manifests"
///         ));
///     }
///     
///     // Safe to delete...
///     Ok(response)
/// }
/// ```
use tracing::{debug, instrument};

use super::super::types::RegistryError;
use crate::prelude::*;

/// Check if a blob is referenced by any manifests.
///
/// This function queries the `container_registry_layer_manifest` table
/// to determine if the specified blob digest is referenced by any manifest.
/// This is crucial for garbage collection and safe deletion operations.
///
/// # Arguments
///
/// * `database` - A mutable reference to the database transaction
/// * `blob_digest` - The SHA256 digest of the blob to check (format:
///   "sha256:...")
///
/// # Returns
///
/// * `Ok(true)` - If the blob is referenced by at least one manifest
/// * `Ok(false)` - If the blob is not referenced by any manifest (safe to
///   delete)
/// * `Err(RegistryError)` - If a database error occurs
///
/// # Examples
///
/// ```ignore
/// let can_delete = !is_blob_referenced(&mut tx, "sha256:abc123...").await?;
/// if can_delete {
///     // Proceed with deletion
/// }
/// ```
///
/// # Requirements
///
/// This function satisfies requirements:
/// - 8.5: Check if blob is referenced by any manifests before deletion
/// - 9.4: Only delete blob if no manifests reference it
/// - 14.2: Identify orphaned blobs for garbage collection
#[instrument(skip(database), fields(blob_digest = %blob_digest))]
pub async fn is_blob_referenced(
	database: &mut DatabaseTransaction,
	blob_digest: &str,
) -> Result<bool, RegistryError> {
	debug!(
		blob_digest = %blob_digest,
		"Checking if blob is referenced by any manifests"
	);

	// Query the container_registry_layer_manifest table to check for references
	let result = query!(
		r#"
		SELECT EXISTS(
			SELECT 1
			FROM container_registry_layer_manifest
			WHERE layer_blob_digest = $1
		) as "exists!"
		"#,
		blob_digest
	)
	.fetch_one(&mut **database)
	.await?;

	let is_referenced = result.exists;

	debug!(
		blob_digest = %blob_digest,
		is_referenced = is_referenced,
		"Blob reference check complete"
	);

	Ok(is_referenced)
}

#[cfg(test)]
mod tests {
	// Note: These tests would require a test database setup
	// For now, we document the expected behavior

	/// Test that a blob referenced by a manifest returns true
	#[test]
	#[ignore = "Requires database setup"]
	fn test_is_blob_referenced_returns_true() {
		// Setup: Insert a manifest and a blob reference
		// let digest = "sha256:abc123...";
		// let result = is_blob_referenced(&mut tx, digest).await.unwrap();
		// assert!(result);
	}

	/// Test that a blob not referenced by any manifest returns false
	#[test]
	#[ignore = "Requires database setup"]
	fn test_is_blob_referenced_returns_false() {
		// Setup: Insert a blob but no manifest references
		// let digest = "sha256:xyz789...";
		// let result = is_blob_referenced(&mut tx, digest).await.unwrap();
		// assert!(!result);
	}

	/// Test that a blob referenced by multiple manifests returns true
	#[test]
	#[ignore = "Requires database setup"]
	fn test_is_blob_referenced_multiple_manifests() {
		// Setup: Insert multiple manifests referencing the same blob
		// let digest = "sha256:def456...";
		// let result = is_blob_referenced(&mut tx, digest).await.unwrap();
		// assert!(result);
	}

	/// Test that checking a non-existent blob returns false
	#[test]
	#[ignore = "Requires database setup"]
	fn test_is_blob_referenced_nonexistent_blob() {
		// Setup: Use a digest that doesn't exist in the database
		// let digest = "sha256:nonexistent...";
		// let result = is_blob_referenced(&mut tx, digest).await.unwrap();
		// assert!(!result);
	}
}
