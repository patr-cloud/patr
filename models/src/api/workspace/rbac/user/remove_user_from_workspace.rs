use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to remove user from a workspace
	RemoveUserFromWorkspace,
	DELETE "/workspace/:workspace_id/rbac/user/:user_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The user ID of the user to add to the workspace
		pub user_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::ModifyRoles,
		}
	}
);
