use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to update the domains DNS record
	VerifyDomainInWorkspace,
	POST "/domain/{domain_id}/verify" {
		/// The domain ID of the record
		pub domain_id: Uuid,
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
			extract_resource_id: |req| req.path.domain_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Domain(DomainPermission::Verify),
		}
	},
	response = {
		/// Whether the domain is verified or not
		pub verified: bool,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Domain,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.domain_id),
	},
);
