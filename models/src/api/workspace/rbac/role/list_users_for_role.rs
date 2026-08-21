use crate::{api::workspace::rbac::user::WorkspaceUserInfo, prelude::*};

macros::declare_api_endpoint!(
	/// Route to list all the users with the role
	ListUsersForRole,
	GET "/workspace/{workspace_id}/rbac/role/{role_id}/users" {
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
	listable_resource = WorkspaceUserInfo,
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ViewRoles,
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of users with the role, with their details
		pub users: Vec<WithId<WorkspaceUserInfo>>
	},
	audit_log = NoAuditLogger,
);
