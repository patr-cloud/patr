use crate::{prelude::*, utils::BearerToken};
use std::collections::BTreeMap;

macros::declare_api_endpoint!(
	/// Route to list all users and their role in a workspace
	ListUsersWithRolesInWorkspace,
	GET "/workspace/:workspace_id/rbac/user" {
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
	response = {
		/// List of all users with thier set of roles in a workspace
		pub users: BTreeMap<Uuid, Vec<Uuid>>,
	},
);
