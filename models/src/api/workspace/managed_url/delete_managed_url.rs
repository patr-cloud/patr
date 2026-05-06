use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a managed URL
	DeleteManagedURL,
	DELETE "/infrastructure/managed-url/{managed_url_id}" {
		/// The manged URL ID to be deleted
		pub managed_url_id: Uuid,
	},
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.managed_url_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ManagedURL(ManagedURLPermission::Delete),
		}
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceDeleted,
		resource_type: ResourceType::ManagedURL,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.managed_url_id),
	},
);
