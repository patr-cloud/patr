use super::ServiceAccount;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all service accounts in a workspace
	ListServiceAccounts,
	GET "/workspace/{workspace_id}/service-account" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	listable_resource = ServiceAccount,
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
		/// The list of service accounts in the workspace
		pub service_accounts: Vec<WithId<ServiceAccount>>,
	},
	audit_log = NoAuditLogger,
);
