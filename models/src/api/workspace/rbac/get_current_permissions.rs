use crate::{prelude::*, rbac::WorkspacePermission};

macros::declare_api_endpoint!(
	/// Route to get current permissions
	GetCurrentPermissions,
	GET "/workspace/:workspace_id/rbac/current-permissions" {
		/// The ID of the workspace
		pub workspace_id: Uuid
	},
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
	response = {
		/// The permissions
		#[serde(flatten)]
		pub permissions: WorkspacePermission
	}
);
