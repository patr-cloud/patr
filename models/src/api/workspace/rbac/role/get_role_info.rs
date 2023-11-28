use crate::{prelude::*, utils::BearerToken};
use std::collections::BTreeMap;
use super::Role;

macros::declare_api_endpoint!(
	/// Route to get the role info
	GetRoleInfo,
	GET "/workspaces/:workspace_id/rbac/role/:role_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The role ID to get the info of
		pub role_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.role_id
		}
	},
	response = {
		/// The role which contains:
		///     name - The role name
		///     description - The role description
		pub role: WithId<Role>,
		/// The list of permission this new role has
		pub resource_permissions: BTreeMap<Uuid, Vec<Uuid>>,
		/// The list of permissions this new role has on what resource types
		pub resource_type_permissions: BTreeMap<Uuid, Vec<Uuid>>,
	}
);
