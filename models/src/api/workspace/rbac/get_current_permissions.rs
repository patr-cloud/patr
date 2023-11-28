use crate::{permission::WorkspacePermission, prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to get current permissions
	GetCurrentPermissions,
	GET "/workspace/:workspace_id/rbac/current-permissions" {
		/// The ID of the workspace
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
	response = {
		/// The permissions
		#[serde(flatten)]
		pub permissions: WorkspacePermission
	}
);
