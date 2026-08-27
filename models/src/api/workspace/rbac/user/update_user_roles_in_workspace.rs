use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to update the roles of a user in a workspace
	UpdateUserRolesInWorkspace,
	POST "/workspace/{workspace_id}/rbac/user/{user_id}" {
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
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ModifyRoles,
		}
	},
	request = {
		/// The role grants the user holds after this call. Empty drops every
		/// grant but keeps the user a member of the workspace.
		#[preprocess(none)]
		pub roles: Vec<super::RoleGrant>,
	},
	audit_logger = NoAuditLogger,
);
