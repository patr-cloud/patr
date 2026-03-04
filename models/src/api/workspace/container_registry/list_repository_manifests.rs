use super::ContainerRepositoryManifestInfo;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get list of all manifests for a container repository.
	ListContainerRepositoryManifests,
	GET "/workspace/{workspace_id}/container-registry/{repository_id}/manifest" {
		/// The workspace ID to list the container registry repositories in
		pub workspace_id: Uuid,
		/// The repository ID to list the manifests for
		pub repository_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id,
			extract_resource_id: |req| req.path.repository_id,
			permission: Permission::ContainerRegistryRepository(
				ContainerRegistryRepositoryPermission::View,
			),
		}
	},
	listable_resource = ContainerRepositoryManifestInfo,
	response_headers = {
		/// The total number of container repositories in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// List of container repositories in the current workspace
		pub manifests: Vec<ContainerRepositoryManifestInfo>,
	},
	audit_log = NoAuditLogger,
);
