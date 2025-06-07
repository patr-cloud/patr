use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all the users with the role
	ListUsersForRole,
	GET "/workspace/:workspace_id/rbac/role/:role_id/users" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the role to get users for
		pub role_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	pagination = true,
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::ViewRoles,
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of users with the role
		pub users: Vec<Uuid>
	}
);
