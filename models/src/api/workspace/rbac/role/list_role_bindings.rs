use crate::{api::workspace::rbac::role::RoleBinding, prelude::*};

macros::declare_api_endpoint!(
	/// Route to list every binding of a role: each member or service account
	/// holding it, and where it applies
	ListRoleBindings,
	GET "/workspace/{workspace_id}/rbac/role/{role_id}/bindings" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the role to list the bindings of
		pub role_id: Uuid
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
			permission: Permission::ViewRoles,
		}
	},
	response = {
		/// Every binding of the role
		pub bindings: Vec<RoleBinding>,
	},
	client_type = [ApiToken, ServiceAccount, WebLogin],
	audit_log = NoAuditLogger,
);
