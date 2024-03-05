use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a role
	DeleteRole,
	DELETE "/workspaces/:workspace_id/rbac/role/:role_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The role ID to delete
		pub role_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.role_id
		}
	}
);
