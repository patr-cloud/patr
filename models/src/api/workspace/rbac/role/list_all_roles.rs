use crate::{prelude::*, utils::BearerToken};
use super::Role;

macros::declare_api_endpoint!(
	/// Route to list all the roles
	ListAllRoles,
	GET "/workspaces/:workspace_id/rbac/role" {
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
		/// The list of all roles that contains:
		///     name - The role name
		///     description - The role description
		pub roles: Vec<WithId<Role>>,
	}
);
