use std::collections::BTreeSet;

use super::Role;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get the role info
	GetRoleInfo,
	GET "/workspace/{workspace_id}/rbac/role/{role_id}" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The role ID to get the info of
		pub role_id: Uuid
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
			permission: Permission::ViewRoles,
		}
	},
	response = {
		/// The role which contains:
		///     name - The role name
		///     description - The role description
		///     isImmutable - Whether the role is a seeded default
		#[serde(flatten)]
		pub role: WithId<Role>,
		/// The permission IDs this role grants.
		pub permissions: BTreeSet<Uuid>,
	},
	audit_log = NoAuditLogger,
);
