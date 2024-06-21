use std::collections::BTreeMap;

use crate::{prelude::*, rbac::ResourcePermissionType, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a new role
	CreateNewRole,
	POST "/workspaces/:workspace_id/rbac/role" {
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
			permission: Permission::ModifyRoles,
		}
	},
	request = {
		/// The name of the new role
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The description of the new role
		#[preprocess(trim)]
		pub description: String,
		/// List of Permission IDs and the type of permission that is granted on this new role.
		#[preprocess(none)]
		pub permissions: BTreeMap<Uuid, ResourcePermissionType>,
	},
	response = {
		/// The ID of the created role
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
