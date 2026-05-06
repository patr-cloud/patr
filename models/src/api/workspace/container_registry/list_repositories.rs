use super::ContainerRepository;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get list of all container repositories in a workspace
	ListContainerRepositories,
	GET "/container-registry",
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id
		}
	},
	listable_resource = ContainerRepository,
	response_headers = {
		/// The total number of container repositories in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// List of container repositories in the current workspace
		pub repositories: Vec<WithId<ContainerRepository>>
	},
	audit_log = NoAuditLogger,
);
