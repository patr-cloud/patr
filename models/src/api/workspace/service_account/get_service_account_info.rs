use super::ServiceAccount;
use crate::{api::workspace::rbac::user::RoleBindingGrant, prelude::*};

macros::declare_api_endpoint!(
	/// Route to get service account information
	GetServiceAccountInfo,
	GET "/workspace/{workspace_id}/service-account/{service_account_id}" {
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
			permission: Permission::ServiceAccount(ServiceAccountPermission::View),
		}
	},
	response = {
		/// The service account's name and description
		#[serde(flatten)]
		pub service_account: WithId<ServiceAccount>,
		/// The role grants this service account holds — each a role and the
		/// resource it applies at.
		pub role_bindings: Vec<RoleBindingGrant>,
	},
	client_type = [WebLogin, ApiToken],
	audit_log = NoAuditLogger,
);
