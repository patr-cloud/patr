use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a role
	DeleteRole,
	DELETE "/workspace/{workspace_id}/rbac/role/{role_id}" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The role ID to delete
		pub role_id: Uuid,
	},
	query = {
		/// Whether to remove the role's bindings. If set to true, every member
		/// and service account holding the role loses it. If set to false, the
		/// role will be deleted only if nobody holds it. By default, this is
		/// set to false.
		#[serde(default)]
		pub remove_bindings: bool,
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
	client_type = [ApiToken, ServiceAccount, WebLogin],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceDeleted,
		resource_type: ResourceType::Role,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.role_id),
	},
);
