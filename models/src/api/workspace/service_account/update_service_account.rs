use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to update a service account
	UpdateServiceAccount,
	PATCH "/workspace/{workspace_id}/service-account/{service_account_id}" {
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
			permission: Permission::ServiceAccount(ServiceAccountPermission::Edit),
		}
	},
	request = {
		/// Updated name
		#[preprocess(optional(trim, regex = RESOURCE_NAME_REGEX))]
		pub name: Option<String>,
		/// Updated description
		#[preprocess(none)]
		pub description: Option<String>,
		/// Updated role assignments (replaces all roles)
		pub roles: Option<Vec<Uuid>>,
	},
	client_type = [WebDashboard, ApiToken],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::ServiceAccount,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.service_account_id),
	},
);
