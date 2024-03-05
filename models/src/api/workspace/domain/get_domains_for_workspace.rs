use super::WorkspaceDomain;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get all the domains in a workspace
	GetDomainsForWorkspace,
	GET "/workspace/:workspace_id/domain" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	pagination = true,
	authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of domains containing:
		/// - domain - The domain metadata
		/// - is_verified - whether the domain is verified or not
		/// - nameserver_type - The type of the nameserver
		pub domains: Vec<WithId<WorkspaceDomain>>,
	}
);
