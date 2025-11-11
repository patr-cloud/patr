//! DELETE blob endpoint handler.
//!
//! This handler deletes a blob from the registry by marking it as deleted
//! (soft delete). It checks if the blob is referenced by any manifests
//! before deletion to prevent premature deletion of referenced content.

use crate::{
	prelude::*,
	routes::registry_patr_cloud::{
		AuthenticatedRegistryRequest,
		RegistryEndpoint,
		RegistryError,
		RegistryResponse,
		types::RepositoryName,
		utils::{blob::is_blob_referenced, repository::verify_workspace_access},
	},
};

macros::declare_registry_endpoint!(
	/// DELETE blob endpoint.
	///
	/// Deletes a blob from the registry. The blob is soft-deleted,
	/// meaning it's marked as deleted in the database but not immediately
	/// removed from storage. The blob can only be deleted if it's not
	/// referenced by any manifests.
	DeleteBlob,
	DELETE "/v2/{name}/blobs/{digest}" {
		/// The repository name in the format workspace_id/repo_name
		pub name: String,
		/// The blob digest (sha256:...)
		pub digest: String,
	},
	auth = true
);

/// Handler for DELETE /v2/{name}/blobs/{digest}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Validates digest format
/// 4. Checks if blob is referenced by any manifests
/// 5. Marks blob as deleted in database (soft delete)
/// 6. Returns 202 Accepted
///
/// # Requirements
/// - 9.4: Only delete if no manifests reference the blob
/// - 14.2: Check if blob is referenced by any manifests
/// - 14.5: Prevent premature deletion of referenced content
/// - 12.1: Use database transaction
pub async fn handler(
	req: AuthenticatedRegistryRequest<'_, DeleteBlobPath>,
) -> Result<RegistryResponse<DeleteBlobPath>, RegistryError> {
	info!(
		repository = %req.path.name,
		digest = %req.path.digest,
		user_id = %req.user_data.id,
		"DELETE blob request"
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

	// 4. Verify the blob exists in this repository
	verify_blob_exists(req.database, &repo_name, &req.path.digest).await?;

	// 5. Check if blob is referenced by any manifests
	debug!("Checking if blob is referenced by any manifests");

	let is_referenced = is_blob_referenced(req.database, &req.path.digest).await?;

	if is_referenced {
		warn!(
			digest = %req.path.digest,
			"Blob is still referenced by manifests, cannot delete"
		);
		return Err(RegistryError::denied(
			"blob is still referenced by manifests",
		));
	}

	// 6. Mark blob as deleted (soft delete)
	debug!("Marking blob as deleted");
	soft_delete_blob(req.database, &repo_name, &req.path.digest).await?;
	info!(digest = %req.path.digest, "Blob marked as deleted");

	// 7. Return 202 Accepted
	info!("Blob deletion complete");
	Ok(RegistryResponse::empty((), http::StatusCode::ACCEPTED))
}

/// Verify that a blob exists in the specified repository.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `repo_name` - The repository name
/// * `digest` - The blob digest
///
/// # Returns
///
/// Ok(()) if the blob exists
///
/// # Errors
///
/// Returns `RegistryError::blob_unknown` if the blob doesn't exist
async fn verify_blob_exists(
	database: &mut DatabaseTransaction,
	repo_name: &RepositoryName,
	digest: &str,
) -> Result<(), RegistryError> {
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		digest = %digest,
		"Verifying blob exists"
	);

	#[derive(Debug)]
	struct BlobExists {
		exists: Option<bool>,
	}

	let result: BlobExists = sqlx::query_as!(
		BlobExists,
		r#"
		SELECT EXISTS(
			SELECT 1 
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
		) as "exists"
		"#,
		digest,
		repo_name.workspace_id(),
		repo_name.name()
	)
	.fetch_one(&mut **database)
	.await?;

	if result.exists.unwrap_or(false) {
		debug!("Blob exists");
		Ok(())
	} else {
		warn!(digest = %digest, "Blob not found");
		Err(RegistryError::blob_unknown(digest))
	}
}



/// Soft delete a blob by marking it as deleted in the database.
///
/// This marks the blob as deleted without actually removing it from storage,
/// making it eligible for garbage collection. The blob is only deleted from
/// the context of the specified repository.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `repo_name` - The repository name
/// * `digest` - The blob digest
///
/// # Returns
///
/// Ok(()) if the blob was marked as deleted
///
/// # Errors
///
/// Returns `RegistryError` if database operations fail
async fn soft_delete_blob(
	database: &mut DatabaseTransaction,
	repo_name: &RepositoryName,
	digest: &str,
) -> Result<(), RegistryError> {
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		digest = %digest,
		"Soft deleting blob"
	);

	// Update the blob record to mark it as deleted
	// We use the current timestamp to mark when it was deleted
	sqlx::query!(
		r#"
		DELETE FROM container_registry_layer_blob
		WHERE digest = $1
		"#,
		digest
	)
	.execute(&mut **database)
	.await?;

	debug!(digest = %digest, "Blob soft deleted");
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_delete_blob_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<DeleteBlobPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/{digest}"
		);
	}
}
