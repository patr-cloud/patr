use std::collections::BTreeMap;

use crate::{
	prelude::*,
	rbac::ResourcePermissionType,
	utils::constants::{RESOURCE_NAME_REGEX, ROLE_DESCRIPTION_REGEX},
};

macros::declare_api_endpoint!(
	/// Route to create a new role
	UpdateRole,
	PATCH "/workspace/{workspace_id}/rbac/role/{role_id}" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the role to update
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
			permission: Permission::ModifyRoles,
		}
	},
	request = {
		/// The updated name of the role
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The updated description of the role
		#[preprocess(trim, regex = ROLE_DESCRIPTION_REGEX)]
		pub description: String,
		/// The updated list of permission this role has
		#[preprocess(none)]
		pub permissions: BTreeMap<Uuid, ResourcePermissionType>,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Role,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.role_id),
	},
);
