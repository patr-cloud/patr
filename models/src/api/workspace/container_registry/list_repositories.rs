use crate::prelude::*;
use super::ContainerRepository;

macros::declare_api_endpoint!(
	/// Route to get list of all container repositories in a workspace
	ListContainerRegistryRepositories,
	GET "/workspace/:workspace_id/container-registry" {
		/// The workspace ID to list the container registry repositories in
		pub workspace_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id
		}
	},
	pagination = true,
	response_headers = {
		/// The total number of container repositories in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// List of container repositories in the current workspace
		pub repositories: Vec<WithId<ContainerRepository>>
	}
);
