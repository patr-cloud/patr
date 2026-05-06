use super::Role;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all the roles
	ListAllRoles,
	GET "/rbac/role",
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	listable_resource = Role,
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
		/// The list of all roles that contains:
		///     name - The role name
		///     description - The role description
		pub roles: Vec<WithId<Role>>,
	},
	audit_log = NoAuditLogger,
);
