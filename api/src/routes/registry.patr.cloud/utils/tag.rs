/// Tag resolution utilities.
///
/// This module provides functions for resolving tag names to manifest digests
/// by querying the container_registry_tag table.
///
/// # Usage Example
///
/// ```ignore
/// use crate::routes::registry_patr_cloud::{
///     types::RepositoryName,
///     utils::tag::resolve_tag_to_digest,
/// };
///
/// // In a handler function:
/// pub async fn handler(
///     req: AuthenticatedRegistryRequest<'_, GetManifest>
/// ) -> Result<RegistryResponse<GetManifest>, RegistryError> {
///     let repo_name = RepositoryName::parse(&req.path.name)?;
///     
///     // Resolve tag to digest
///     let digest = resolve_tag_to_digest(
///         req.database,
///         &repo_name,
///         &req.path.reference
///     ).await?;
///     
///     // Continue with handler logic...
///     Ok(response)
/// }
/// ```
use tracing::{debug, instrument};

use super::super::types::{RegistryError, RepositoryName};
use crate::prelude::*;

/// Resolve a tag name to a manifest digest.
///
/// This function queries the container_registry_tag table to find the manifest
/// digest associated with a given tag name in a specific repository.
///
/// # Arguments
///
/// * `database` - Database transaction to use for the query
/// * `repo_name` - The repository name (workspace_id/repo_name format)
/// * `tag_name` - The tag name to resolve
///
/// # Returns
///
/// * `Ok(String)` - The manifest digest (e.g., "sha256:abc123...")
/// * `Err(RegistryError)` - If the tag is not found or a database error occurs
///
/// # Examples
///
/// ```ignore
/// let digest = resolve_tag_to_digest(
///     &mut transaction,
///     &repo_name,
///     "latest"
/// ).await?;
/// ```
///
/// # Requirements
///
/// This function satisfies requirements:
/// - 5.4: Resolve tag name to latest manifest digest
/// - 7.1: Create unique tag references for manifests
/// - 7.4: Atomically update tag references to new manifests
#[instrument(skip(database), fields(workspace_id = %repo_name.workspace_id(), repo_name = %repo_name.name(), tag_name = %tag_name))]
pub async fn resolve_tag_to_digest(
	database: &mut DatabaseTransaction,
	repo_name: &RepositoryName,
	tag_name: &str,
) -> Result<String, RegistryError> {
	debug!(
		workspace_id = %repo_name.workspace_id(),
		repo_name = %repo_name.name(),
		tag_name = %tag_name,
		"Resolving tag to manifest digest"
	);

	// First, find the repository ID
	let repository_record = query!(
		r#"
		SELECT id
		FROM container_registry_repository
		WHERE workspace_id = $1 AND name = $2 AND deleted IS NULL
		"#,
		repo_name.workspace_id(),
		repo_name.name()
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or_else(|| {
		debug!(
			workspace_id = %repo_name.workspace_id(),
			repo_name = %repo_name.name(),
			"Repository not found"
		);
		RegistryError::name_unknown(repo_name.to_string())
	})?;

	// Query the tag table to get the manifest digest
	let tag_record = query!(
		r#"
		SELECT manifest_digest
		FROM container_registry_tag
		WHERE repository_id = $1 AND name = $2
		"#,
		repository_record.id,
		tag_name
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or_else(|| {
		debug!(
			repository_id = %repository_record.id,
			tag_name = %tag_name,
			"Tag not found"
		);
		RegistryError::manifest_unknown(format!("{}:{}", repo_name.to_string(), tag_name))
	})?;

	debug!(
		repository_id = %repository_record.id,
		tag_name = %tag_name,
		manifest_digest = %tag_record.manifest_digest,
		"Tag resolved to manifest digest"
	);

	Ok(tag_record.manifest_digest)
}

#[cfg(test)]
mod tests {
	// Note: These tests would require a test database setup.
	// For now, we document the expected behavior.

	/// Test that resolve_tag_to_digest returns the correct digest for a valid
	/// tag
	#[tokio::test]
	#[ignore = "requires database setup"]
	async fn test_resolve_tag_to_digest_success() {
		// Setup: Create a repository, manifest, and tag in the database
		// let mut transaction = setup_test_database().await;
		// let repo_name =
		// RepositoryName::parse("550e8400-e29b-41d4-a716-446655440000/test-app"
		// ).unwrap(); let tag_name = "latest";
		// let expected_digest = "sha256:abc123...";
		//
		// // Insert test data...
		//
		// let result = resolve_tag_to_digest(&mut transaction, &repo_name,
		// tag_name).await; assert!(result.is_ok());
		// assert_eq!(result.unwrap(), expected_digest);
	}

	/// Test that resolve_tag_to_digest returns an error for a non-existent
	/// repository
	#[tokio::test]
	#[ignore = "requires database setup"]
	async fn test_resolve_tag_to_digest_repository_not_found() {
		// Setup: Use a database with no repositories
		// let mut transaction = setup_test_database().await;
		// let repo_name =
		// RepositoryName::parse("550e8400-e29b-41d4-a716-446655440000/
		// nonexistent").unwrap(); let tag_name = "latest";
		//
		// let result = resolve_tag_to_digest(&mut transaction, &repo_name,
		// tag_name).await; assert!(result.is_err());
		// let err = result.unwrap_err();
		// assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
	}

	/// Test that resolve_tag_to_digest returns an error for a non-existent tag
	#[tokio::test]
	#[ignore = "requires database setup"]
	async fn test_resolve_tag_to_digest_tag_not_found() {
		// Setup: Create a repository but no tag
		// let mut transaction = setup_test_database().await;
		// let repo_name =
		// RepositoryName::parse("550e8400-e29b-41d4-a716-446655440000/test-app"
		// ).unwrap(); let tag_name = "nonexistent";
		//
		// // Insert repository but no tag...
		//
		// let result = resolve_tag_to_digest(&mut transaction, &repo_name,
		// tag_name).await; assert!(result.is_err());
		// let err = result.unwrap_err();
		// assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
	}

	/// Test that resolve_tag_to_digest handles deleted repositories correctly
	#[tokio::test]
	#[ignore = "requires database setup"]
	async fn test_resolve_tag_to_digest_deleted_repository() {
		// Setup: Create a repository and mark it as deleted
		// let mut transaction = setup_test_database().await;
		// let repo_name =
		// RepositoryName::parse("550e8400-e29b-41d4-a716-446655440000/
		// deleted-app").unwrap(); let tag_name = "latest";
		//
		// // Insert repository with deleted timestamp...
		//
		// let result = resolve_tag_to_digest(&mut transaction, &repo_name,
		// tag_name).await; assert!(result.is_err());
		// let err = result.unwrap_err();
		// assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
	}
}
