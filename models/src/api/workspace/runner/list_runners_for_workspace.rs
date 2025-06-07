use super::Runner;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all the runners of a workspace
	ListRunnersForWorkspace,
	GET "/workspace/:workspace_id/runner" {
		/// The ID of the workspace
		pub workspace_id: Uuid
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
			extract_workspace_id: |req| req.path.workspace_id,
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of runners in the workspace
		pub runners: Vec<WithId<Runner>>,
	}
);
