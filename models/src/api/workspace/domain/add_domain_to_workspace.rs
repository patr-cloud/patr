use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to add a domain to a workspace. Patr only validates ownership.
	AddDomainToWorkspace,
	POST "/workspace/{workspace_id}/domain" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
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
			permission: Permission::Domain(DomainPermission::Add),
		}
	},
	request = {
		/// The name of the domain
		#[preprocess(domain)]
		pub domain: String,
	},
	response = {
		/// The ID of the created record
		#[serde(flatten)]
		pub id: OnlyId,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceCreated,
		resource_type: ResourceType::Domain,
		extract_resource_id: ResourceIdExtractor::FromResponse(|res| res.body.id.id),
	},
);
