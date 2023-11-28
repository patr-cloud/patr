use crate::{prelude::*, utils::BearerToken};
use std::collections::BTreeMap;

macros::declare_api_endpoint!(
	/// Route to create a new role
	UpdateRole,
	PATCH "/workspace/:workspace_id/rbac/role/:role_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the role to update
		pub role_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// The updated name of the role
		pub name: Option<String>,
		/// The updated description of the role
		pub description: Option<String>,
		/// The updated list of permission this role has
		pub resource_permissions: Option<BTreeMap<Uuid, Vec<Uuid>>>,
		/// The updated list of permissions this role has on what resource types
		pub resource_type_permissions: Option<BTreeMap<Uuid, Vec<Uuid>>>
	}
);
