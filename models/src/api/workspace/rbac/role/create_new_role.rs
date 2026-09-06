use std::collections::BTreeSet;

use super::Role;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to create a new role
	CreateNewRole,
	POST "/workspace/{workspace_id}/rbac/role" {
		/// The ID of the workspace
		pub workspace_id: Uuid
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
	request = {
		/// The name and description of the new role
		#[serde(flatten)]
		#[preprocess]
		pub role: Role,
		/// The permission IDs this role grants; targeting lives on the
		/// binding, not the role.
		#[preprocess(none)]
		pub permissions: BTreeSet<Uuid>,
	},
	response = {
		/// The ID of the created role
		#[serde(flatten)]
		pub id: OnlyId,
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceCreated,
		resource_type: ResourceType::Role,
		extract_resource_id: ResourceIdExtractor::FromResponse(|res| res.body.id.id),
	},
);
