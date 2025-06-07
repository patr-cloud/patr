use super::ContainerRepository;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Gets the information of a container repository in the workspace.
	GetContainerRepositoryInfo,
	GET "/workspace/:workspace_id/container-registry/:repository_id" {
		/// The workspace to get the container repository in.
		pub workspace_id: Uuid,
		/// The id of the repository to get the information of.
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
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View),
		}
	},
	response = {
		/// The information of the container repository.
		pub repository: ContainerRepository,
	}
);
