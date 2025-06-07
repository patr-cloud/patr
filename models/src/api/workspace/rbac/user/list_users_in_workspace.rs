use std::collections::BTreeMap;

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all users and their role in a workspace
	ListUsersInWorkspace,
	GET "/workspace/:workspace_id/rbac/user" {
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
		/// List of all users with their set of roles in a workspace
		pub users: BTreeMap<Uuid, Vec<Uuid>>,
	},
);
