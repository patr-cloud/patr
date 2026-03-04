use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete domain in a workspace
	DeleteDomainInWorkspace,
	DELETE "/workspace/{workspace_id}/domain/{domain_id}" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID of the workspace
		pub domain_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.domain_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Domain(DomainPermission::Delete),
		}
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceDeleted,
		resource_type: ResourceType::Domain,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.domain_id),
	},
);
