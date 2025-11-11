//! DELETE manifest endpoint handler.
//!
//! This handler deletes a manifest from the registry by marking it as deleted
//! (soft delete). It checks if the manifest is referenced by other tags or
//! manifests before deletion.

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// DELETE manifest endpoint.
	///
	/// Deletes a manifest from the registry. The manifest is soft-deleted,
	/// meaning it's marked as deleted in the database but not immediately
	/// removed from storage.
	DeleteManifest,
	DELETE "/v2/{workspace_id}/{repo_name}/manifests/{reference}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = "^[a-z0-9]+([._-][a-z0-9]+)*$", length(max = 255))]
		pub repo_name: String,
		/// The manifest reference (tag name or digest)
		#[preprocess(regex = "^[A-Za-z0-9._\\+-]+(:[A-Za-z0-9._\\=-]+)?$")]
		pub reference: String,
	},
	auth = true,
	request_headers = {
		/// Authorization header with Bearer token
		pub authorization: BearerToken,
	},
);

/// Handler for DELETE /v2/{name}/manifests/{reference}
///
/// This handler:
/// 1. Parses and validates the repository name
/// 2. Verifies workspace access
/// 3. Resolves the reference (tag or digest) to a manifest digest
/// 4. Checks if manifest is referenced by other tags or manifests
/// 5. Marks manifest as deleted in database (soft delete)
/// 6. Returns 202 Accepted
///
/// # Requirements
/// - 8.5: Remove only the tag reference, not the manifest (if reference is a
///   tag)
/// - 14.1: Mark manifest as eligible for garbage collection
/// - 14.5: Prevent premature deletion of referenced content
/// - 12.1: Use database transaction
pub async fn delete_manifest(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path:
					DeleteManifestPathProcessed {
						workspace_id,
						repo_name,
						reference,
					},
				query,
				headers,
				body,
			},
		database,
		redis,
		s3,
		client_ip,
		user_data,
		config,
	}: AuthenticatedRegistryAppRequest<'_, DeleteManifestPath>,
) -> Result<RegistryResponse<DeleteManifestPath>, RegistryError> {
	info!(
		repository = %repo_name,
		reference = %reference,
		user_id = %user_data.id,
		"DELETE manifest handler invoked"
	);

	// 1. Parse repository name
	debug!(
		workspace_id = %workspace_id,
		repo_name = %repo_name,
		"Parsed repository name"
	);

	// 2. Verify workspace access
	if !user_data.has_resource_permission(workspace_id, resource_id, required_permission) {
		return RegistryError::builder()
			.code(ErrorCode::ManifestBlobUnknown)
			.message("Workspace does not exist or access denied")
			.status(StatusCode::NOT_FOUND)
			.build()
			.into_result();
	}
	debug!("Workspace access verified");

	// 3. Resolve reference to manifest digest
	let manifest_digest = if reference.contains(":") {
		// Reference is already a digest
		debug!("Reference is a digest");
		path.reference.clone()
	} else {
		// Reference is a tag, need to resolve it
		debug!("Reference is a tag, resolving to digest");
		resolve_tag_to_digest(database, &repo_name, &path.reference).await?
	};

	debug!(digest = %manifest_digest, "Resolved manifest digest");

	// 4. Verify the manifest exists in this repository
	verify_manifest_exists(database, &repo_name, &manifest_digest).await?;

	// 5. If reference is a tag, delete the tag (not the manifest)
	if !path.reference.starts_with("sha256:") {
		debug!(tag = %path.reference, "Deleting tag reference");
		delete_tag(database, &repo_name, &path.reference).await?;
		info!(tag = %path.reference, "Tag deleted");
	} else {
		// Reference is a digest, check if manifest is referenced by other tags or
		// manifests
		debug!("Checking if manifest is referenced by other tags or manifests");

		let is_referenced = is_manifest_referenced(database, &repo_name, &manifest_digest).await?;

		if is_referenced {
			warn!(
				digest = %manifest_digest,
				"Manifest is still referenced by other tags or manifests, cannot delete"
			);
			return Err(RegistryError::denied(
				"manifest is still referenced by tags or other manifests",
			));
		}

		// 6. Mark manifest as deleted (soft delete)
		debug!("Marking manifest as deleted");
		soft_delete_manifest(database, &repo_name, &manifest_digest).await?;
		info!(digest = %manifest_digest, "Manifest marked as deleted");
	}

	// 7. Return 202 Accepted
	info!("Manifest deletion complete");
	RegistryResponse::builder()
		.body(Body::empty())
		.status(StatusCode::ACCEPTED)
		.build()
		.into_result()
}

/// Verify that a manifest exists in the specified repository.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `repo_name` - The repository name
/// * `digest` - The manifest digest
///
/// # Returns
///
/// Ok(()) if the manifest exists
///
/// # Errors
///
/// Returns `RegistryError::manifest_unknown` if the manifest doesn't exist
async fn verify_manifest_exists(
	database: &mut DatabaseTransaction,
	repo_name: &RepositoryName,
	digest: &str,
) -> Result<(), RegistryError> {
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		digest = %digest,
		"Verifying manifest exists"
	);

	#[derive(Debug)]
	struct ManifestExists {
		exists: Option<bool>,
	}

	let result: ManifestExists = sqlx::query_as!(
		ManifestExists,
		r#"
		SELECT EXISTS(
			SELECT 1 
			FROM container_registry_manifest m
			INNER JOIN container_registry_repository_manifest rm 
				ON m.digest = rm.manifest_digest
			INNER JOIN container_registry_repository r 
				ON rm.repository_id = r.id
			WHERE m.digest = $1
				AND r.workspace_id = $2
				AND r.name = $3
				AND r.deleted IS NULL
		) as "exists"
		"#,
		digest,
		repo_name.workspace_id() as _,
		repo_name.name()
	)
	.fetch_one(&mut **database)
	.await?;

	if result.exists.unwrap_or(false) {
		debug!("Manifest exists");
		Ok(())
	} else {
		warn!(digest = %digest, "Manifest not found");
		Err(RegistryError::manifest_unknown(digest))
	}
}

/// Delete a tag from the repository.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `repo_name` - The repository name
/// * `tag` - The tag name to delete
///
/// # Returns
///
/// Ok(()) if the tag was deleted
///
/// # Errors
///
/// Returns `RegistryError` if database operations fail
async fn delete_tag(
	database: &mut DatabaseTransaction,
	repo_name: &RepositoryName,
	tag: &str,
) -> Result<(), RegistryError> {
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		tag = %tag,
		"Deleting tag"
	);

	sqlx::query!(
		r#"
		DELETE FROM container_registry_tag t
		USING container_registry_repository r
		WHERE t.repository_id = r.id
			AND r.workspace_id = $1
			AND r.name = $2
			AND t.name = $3
			AND r.deleted IS NULL
		"#,
		repo_name.workspace_id() as _,
		repo_name.name(),
		tag
	)
	.execute(&mut **database)
	.await?;

	debug!(tag = %tag, "Tag deleted");
	Ok(())
}

/// Check if a manifest is referenced by other tags or manifests.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `repo_name` - The repository name
/// * `digest` - The manifest digest
///
/// # Returns
///
/// True if the manifest is referenced, false otherwise
///
/// # Errors
///
/// Returns `RegistryError` if database operations fail
async fn is_manifest_referenced(
	database: &mut DatabaseTransaction,
	repo_name: &RepositoryName,
	digest: &str,
) -> Result<bool, RegistryError> {
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		digest = %digest,
		"Checking if manifest is referenced"
	);

	// Check if any tags reference this manifest
	#[derive(Debug)]
	struct TagCount {
		count: Option<i64>,
	}

	let tag_count: TagCount = sqlx::query_as!(
		TagCount,
		r#"
		SELECT COUNT(*) as "count"
		FROM container_registry_tag t
		INNER JOIN container_registry_repository r ON t.repository_id = r.id
		WHERE t.manifest_digest = $1
			AND r.workspace_id = $2
			AND r.name = $3
			AND r.deleted IS NULL
		"#,
		digest,
		repo_name.workspace_id() as _,
		repo_name.name()
	)
	.fetch_one(&mut **database)
	.await?;

	if tag_count.count.unwrap_or(0) > 0 {
		debug!(
			count = tag_count.count.unwrap_or(0),
			"Manifest is referenced by tags"
		);
		return Ok(true);
	}

	// Check if any other manifests reference this manifest (for image indexes)
	#[derive(Debug)]
	struct ManifestCount {
		count: Option<i64>,
	}

	let manifest_count: ManifestCount = sqlx::query_as!(
		ManifestCount,
		r#"
		SELECT COUNT(*) as "count"
		FROM container_registry_layer_manifest lm
		WHERE lm.layer_blob_digest = $1
		"#,
		digest
	)
	.fetch_one(&mut **database)
	.await?;

	if manifest_count.count.unwrap_or(0) > 0 {
		debug!(
			count = manifest_count.count.unwrap_or(0),
			"Manifest is referenced by other manifests"
		);
		return Ok(true);
	}

	debug!("Manifest is not referenced");
	Ok(false)
}

/// Soft delete a manifest by removing it from the repository-manifest link
/// table.
///
/// This marks the manifest as deleted without actually removing it from
/// storage, making it eligible for garbage collection.
///
/// # Arguments
///
/// * `database` - Database transaction
/// * `repo_name` - The repository name
/// * `digest` - The manifest digest
///
/// # Returns
///
/// Ok(()) if the manifest was marked as deleted
///
/// # Errors
///
/// Returns `RegistryError` if database operations fail
async fn soft_delete_manifest(
	database: &mut DatabaseTransaction,
	repo_name: &RepositoryName,
	digest: &str,
) -> Result<(), RegistryError> {
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		digest = %digest,
		"Soft deleting manifest"
	);

	// Remove the link between repository and manifest
	// This makes the manifest eligible for garbage collection
	sqlx::query!(
		r#"
		DELETE FROM container_registry_repository_manifest rm
		USING container_registry_repository r
		WHERE rm.repository_id = r.id
			AND rm.manifest_digest = $1
			AND r.workspace_id = $2
			AND r.name = $3
			AND r.deleted IS NULL
		"#,
		digest,
		repo_name.workspace_id() as _,
		repo_name.name()
	)
	.execute(&mut **database)
	.await?;

	debug!(digest = %digest, "Manifest soft deleted");
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_delete_manifest_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<DeleteManifestPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/manifests/{reference}"
		);
	}
}
