/// Repository access validation utilities.
///
/// This module provides functions for verifying that users have
/// access to workspaces and repositories.
///
/// # Usage Example
///
/// ```ignore
/// use crate::routes::registry_patr_cloud::{
///     types::RepositoryName,
///     utils::repository::verify_workspace_access,
/// };
///
/// // In a handler function:
/// pub async fn handler(
///     req: AuthenticatedRegistryRequest<'_, SomeEndpoint>
/// ) -> Result<RegistryResponse<SomeEndpoint>, RegistryError> {
///     // Parse repository name from path
///     let repo_name = RepositoryName::parse(&req.path.name)?;
///     
///     // Verify user has access to the workspace
///     verify_workspace_access(&req.user_data, repo_name.workspace_id())?;
///     
///     // Continue with handler logic...
///     Ok(response)
/// }
/// ```

use models::RequestUserData;
use tracing::debug;

use crate::prelude::*;

use super::super::types::RegistryError;

/// Verify that a user has access to a workspace.
///
/// This function checks if the user is either a SuperAdmin or a Member
/// of the specified workspace. It's used to enforce workspace isolation
/// in the registry, ensuring users can only access repositories within
/// workspaces they belong to.
///
/// # Arguments
///
/// * `user_data` - The authenticated user's data containing workspace permissions
/// * `workspace_id` - The workspace ID to check access for
///
/// # Returns
///
/// * `Ok(())` - If the user has access to the workspace
/// * `Err(RegistryError)` - If the user does not have access (DENIED error)
///
/// # Examples
///
/// ```ignore
/// verify_workspace_access(&req.user_data, repo_name.workspace_id())?;
/// ```
///
/// # Requirements
///
/// This function satisfies requirements:
/// - 3.2: Validate workspace membership when accessing repositories
/// - 3.5: Only show repositories within user's accessible workspaces
/// - 4.3: Extract user and workspace permissions from valid API token
/// - 10.4: Return appropriate error if access denied
pub fn verify_workspace_access(
	user_data: &RequestUserData,
	workspace_id: Uuid,
) -> Result<(), RegistryError> {
	debug!(
		user_id = %user_data.id,
		workspace_id = %workspace_id,
		"Verifying workspace access"
	);

	// Check if the user has any permissions for this workspace
	match user_data.permissions.get(&workspace_id) {
		Some(_permission) => {
			// User has access to this workspace (either SuperAdmin or Member)
			debug!(
				user_id = %user_data.id,
				workspace_id = %workspace_id,
				"Workspace access granted"
			);
			Ok(())
		}
		None => {
			// User does not have access to this workspace
			debug!(
				user_id = %user_data.id,
				workspace_id = %workspace_id,
				"Workspace access denied - user is not a member"
			);
			Err(RegistryError::denied(format!(
				"Access denied: user does not have access to workspace {}",
				workspace_id
			)))
		}
	}
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;

	use models::rbac::WorkspacePermission;
	use time::OffsetDateTime;

	use super::*;

	fn create_test_user_data(workspace_permissions: BTreeMap<Uuid, WorkspacePermission>) -> RequestUserData {
		RequestUserData {
			id: Uuid::new_v4(),
			username: "testuser".to_string(),
			first_name: "Test".to_string(),
			last_name: "User".to_string(),
			created: OffsetDateTime::now_utc(),
			login_id: Uuid::new_v4(),
			permissions: workspace_permissions,
		}
	}

	#[test]
	fn test_verify_workspace_access_super_admin() {
		let workspace_id = Uuid::new_v4();
		let mut permissions = BTreeMap::new();
		permissions.insert(workspace_id, WorkspacePermission::SuperAdmin);

		let user_data = create_test_user_data(permissions);

		let result = verify_workspace_access(&user_data, workspace_id);
		assert!(result.is_ok());
	}

	#[test]
	fn test_verify_workspace_access_member() {
		let workspace_id = Uuid::new_v4();
		let mut permissions = BTreeMap::new();
		permissions.insert(
			workspace_id,
			WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			},
		);

		let user_data = create_test_user_data(permissions);

		let result = verify_workspace_access(&user_data, workspace_id);
		assert!(result.is_ok());
	}

	#[test]
	fn test_verify_workspace_access_denied() {
		let workspace_id = Uuid::new_v4();
		let other_workspace_id = Uuid::new_v4();
		let mut permissions = BTreeMap::new();
		permissions.insert(other_workspace_id, WorkspacePermission::SuperAdmin);

		let user_data = create_test_user_data(permissions);

		let result = verify_workspace_access(&user_data, workspace_id);
		assert!(result.is_err());

		if let Err(err) = result {
			assert_eq!(err.status_code(), axum::http::StatusCode::FORBIDDEN);
		}
	}

	#[test]
	fn test_verify_workspace_access_no_permissions() {
		let workspace_id = Uuid::new_v4();
		let permissions = BTreeMap::new();

		let user_data = create_test_user_data(permissions);

		let result = verify_workspace_access(&user_data, workspace_id);
		assert!(result.is_err());
	}

	#[test]
	fn test_verify_workspace_access_multiple_workspaces() {
		let workspace_id_1 = Uuid::new_v4();
		let workspace_id_2 = Uuid::new_v4();
		let workspace_id_3 = Uuid::new_v4();

		let mut permissions = BTreeMap::new();
		permissions.insert(workspace_id_1, WorkspacePermission::SuperAdmin);
		permissions.insert(
			workspace_id_2,
			WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			},
		);

		let user_data = create_test_user_data(permissions);

		// Should have access to workspace 1
		assert!(verify_workspace_access(&user_data, workspace_id_1).is_ok());

		// Should have access to workspace 2
		assert!(verify_workspace_access(&user_data, workspace_id_2).is_ok());

		// Should NOT have access to workspace 3
		assert!(verify_workspace_access(&user_data, workspace_id_3).is_err());
	}
}
