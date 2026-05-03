use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a service account in a workspace. Returns the service
	/// account ID and the generated token (shown only once).
	CreateServiceAccount,
	POST "/workspace/{workspace_id}/service-account" {
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
			permission: Permission::ServiceAccount(ServiceAccountPermission::Create),
		}
	},
	request = {
		/// Name of the service account
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// Optional description
		#[preprocess(none)]
		pub description: Option<String>,
		/// Roles to assign to this service account
		pub roles: Vec<Uuid>,
	},
	response = {
		/// The ID of the created service account
		#[serde(flatten)]
		pub id: OnlyId,
		/// The generated token (shown only once)
		pub token: String,
	},
	client_type = [WebDashboard, ApiToken],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceCreated,
		resource_type: ResourceType::ServiceAccount,
		extract_resource_id: ResourceIdExtractor::FromResponse(|res| res.body.id.id),
	},
);
