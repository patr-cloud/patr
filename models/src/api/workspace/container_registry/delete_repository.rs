use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Deletes a container repository in the workspace.
	DeleteContainerRepository,
	DELETE "/workspace/{workspace_id}/container-registry/{repository_id}" {
		/// The workspace to delete the container repository in.
		pub workspace_id: Uuid,
		/// The id of the repository to delete.
		pub repository_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.repository_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Delete),
		}
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceDeleted,
		resource_type: ResourceType::ContainerRegistryRepository,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.repository_id),
	},
);
