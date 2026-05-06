use super::ManagedUrlType;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to update a managed URL configurations
	UpdateManagedURL,
	POST "/infrastructure/managed-url/{managed_url_id}" {
		/// The managed URL to be deleted
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
			permission: Permission::ManagedURL(ManagedURLPermission::Edit),
		}
	},
	request = {
		/// The new path of the updated URL
		#[preprocess(optional(trim, lowercase))]
		pub path: Option<String>,
		/// The new type of the updated URL which can be
		/// Deployment, Static Site, Proxy or Redirect
		#[serde(flatten)]
		#[preprocess(optional(none))]
		pub url_type: Option<ManagedUrlType>,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::ManagedURL,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.managed_url_id),
	},
);
