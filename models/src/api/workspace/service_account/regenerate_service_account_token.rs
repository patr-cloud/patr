use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to regenerate a service account's token. The old token is
	/// immediately invalidated. The new token is returned and shown only once.
	RegenerateServiceAccountToken,
	POST "/workspace/{workspace_id}/service-account/{service_account_id}/token" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the service account
		pub service_account_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.service_account_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ServiceAccount(ServiceAccountPermission::RegenerateToken),
		}
	},
	response = {
		/// The new token (shown only once)
		pub token: String,
	},
	client_type = [WebDashboard, ApiToken],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::ServiceAccount,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.service_account_id),
	},
);
