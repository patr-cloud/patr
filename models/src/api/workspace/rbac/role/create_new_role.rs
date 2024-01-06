use std::collections::BTreeMap;

use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to create a new role
	CreateNewRole,
	POST "/workspaces/:workspace_id/rbac/role" {
		/// The ID of the workspace
		pub workspace_id: Uuid
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
		/// The name of the new role
		pub name: String,
		/// The description of the new role
		pub description: String,
		/// The list of permission this new role has
		pub resource_permissions: BTreeMap<Uuid, Vec<Uuid>>,
		/// The list of permissions this new role has on what resource types
		pub resource_type_permissions: BTreeMap<Uuid, Vec<Uuid>>
	},
	response = {
		/// The ID of the created role
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
